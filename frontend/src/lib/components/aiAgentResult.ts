import type { FlowStatusModule } from '$lib/gen'

/** The `agent_action` tag the worker puts on every message it records. */
export type AgentAction = NonNullable<FlowStatusModule['agent_actions']>[number]

export type AgentTokenUsage = {
	input_tokens?: number
	output_tokens?: number
	total_tokens?: number
	cache_read_input_tokens?: number
	cache_write_input_tokens?: number
}

export type AgentMessage = {
	role: string
	content?: unknown
	tool_calls?: Array<{
		id?: string
		type?: string
		function?: { name?: string; arguments?: string }
	}>
	tool_call_id?: string
	agent_action?: AgentAction
	annotations?: Array<{ url: string; title?: string; start_index?: number; end_index?: number }>
}

/** The envelope every AI agent step returns, built by `AIAgentResult`. */
export type AgentResult = {
	output: unknown
	messages: AgentMessage[]
	usage?: AgentTokenUsage
	wm_stream?: string
}

/** Every key `AIAgentResult` can serialize. `usage` and `wm_stream` are skipped when empty. */
const ENVELOPE_KEYS = ['output', 'messages', 'usage', 'wm_stream']

function isRecord(value: unknown): value is Record<string, unknown> {
	return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function hasRole(message: unknown): boolean {
	return isRecord(message) && typeof message.role === 'string'
}

/**
 * A job result is whatever its script returned, so a message that passed the
 * shape check still has arbitrary anything underneath. Everything downstream
 * reads these as `AgentMessage`, and a value of the wrong type there throws in
 * the middle of rendering, taking the whole result viewer with it.
 *
 * So the coercion happens once, here: past this point the declared type is the
 * real one, and no reader needs a guard of its own.
 */
function toAgentMessage(raw: Record<string, unknown>): AgentMessage {
	const toolCalls = Array.isArray(raw.tool_calls)
		? raw.tool_calls.filter(isRecord).map((call) => ({
				id: typeof call.id === 'string' ? call.id : undefined,
				type: typeof call.type === 'string' ? call.type : undefined,
				function: isRecord(call.function)
					? {
							name: typeof call.function.name === 'string' ? call.function.name : undefined,
							arguments:
								typeof call.function.arguments === 'string' ? call.function.arguments : undefined
						}
					: undefined
			}))
		: undefined
	const annotations = Array.isArray(raw.annotations)
		? raw.annotations.filter((a): a is Record<string, unknown> => isRecord(a) && typeof a.url === 'string')
		: undefined
	return {
		role: raw.role as string,
		content: raw.content,
		tool_calls: toolCalls,
		tool_call_id: typeof raw.tool_call_id === 'string' ? raw.tool_call_id : undefined,
		// The union is discriminated on `type`; an action without a string one
		// matches no branch and is treated as untagged.
		agent_action:
			isRecord(raw.agent_action) && typeof raw.agent_action.type === 'string'
				? (raw.agent_action as unknown as AgentAction)
				: undefined,
		annotations: annotations as AgentMessage['annotations']
	}
}

function toAgentMessages(raw: unknown[]): AgentMessage[] {
	return raw.map((message) => toAgentMessage(message as Record<string, unknown>))
}

/**
 * Recognise the envelope by its shape rather than by a marker key the worker
 * would have to add: sniffing works on runs that already completed, and the
 * envelope is also what a nested agent hands back, where an added key would
 * travel into the parent's conversation.
 *
 * The signature is deliberately closed — no key outside `ENVELOPE_KEYS`, and
 * every message carrying a `role` — so an ordinary result that happens to have
 * an `output` field cannot claim it.
 *
 * It stops short of also requiring a recognised `agent_action`, which would rule
 * out a hand-written script returning this same shape. Not every completed run
 * is guaranteed to tag a message (a run whose provider returns its answer
 * through a structured-output tool leaves the final assistant message untagged),
 * and the two failures are not symmetric: claiming a lookalike costs a viewer
 * one click on the JSON toggle, while rejecting a real agent hides its answer
 * with nothing on screen to say why.
 */
export function parseAgentResult(result: unknown): AgentResult | undefined {
	if (!isRecord(result)) {
		return undefined
	}
	const keys = Object.keys(result)
	if (!keys.every((key) => ENVELOPE_KEYS.includes(key))) {
		return undefined
	}
	if (!('output' in result) || !Array.isArray(result.messages)) {
		return undefined
	}
	// An agent always records at least the message it was asked, so an empty list
	// is someone else's result rather than a run that did nothing.
	if (result.messages.length === 0 || !result.messages.every(hasRole)) {
		return undefined
	}
	return {
		output: result.output,
		messages: toAgentMessages(result.messages),
		usage: isRecord(result.usage) ? (result.usage as AgentTokenUsage) : undefined,
		wm_stream: typeof result.wm_stream === 'string' ? result.wm_stream : undefined
	}
}

/**
 * A run stopped by `max_iterations` fails, so it returns an error rather than an
 * envelope — but the worker attaches the conversation so far to it. That partial
 * transcript is the whole reason to look at a run that hit the cap.
 */
export function parseAgentErrorMessages(result: unknown): AgentMessage[] | undefined {
	if (!isRecord(result) || !isRecord(result.error)) {
		return undefined
	}
	const inner = result.error.result
	if (!isRecord(inner) || !Array.isArray(inner.messages)) {
		return undefined
	}
	if (inner.messages.length === 0 || !inner.messages.every(hasRole)) {
		return undefined
	}
	return toAgentMessages(inner.messages)
}

export type AgentResultSummary = {
	toolCalls: number
	webSearches: number
	tokens: number | undefined
	cachedTokens: number | undefined
}

function actionType(message: AgentMessage): string | undefined {
	return message.agent_action?.type
}

export function summarizeAgentResult(result: AgentResult): AgentResultSummary {
	let toolCalls = 0
	let webSearches = 0
	for (const message of result.messages) {
		const type = actionType(message)
		if (type === 'tool_call' || type === 'mcp_tool_call') {
			toolCalls++
		} else if (type === 'web_search') {
			webSearches++
		}
	}
	const usage = result.usage
	// `total_tokens` is what providers report when they report anything; fall back
	// to the parts so a provider that only sends the split still shows a count.
	const tokens =
		usage?.total_tokens ??
		(usage?.input_tokens !== undefined || usage?.output_tokens !== undefined
			? (usage?.input_tokens ?? 0) + (usage?.output_tokens ?? 0)
			: undefined)
	return {
		toolCalls,
		webSearches,
		tokens,
		cachedTokens: usage?.cache_read_input_tokens
	}
}

export type AgentStream = {
	answer: string
	reasoning: string
	/** The most recent tool the run touched, so a stream that is mid-call says so. */
	tool?: { name: string; running: boolean; success?: boolean }
}

/** How much of the stream has been folded in, so the next poll starts there. */
export type AgentStreamProgress = { consumed: number; stream: AgentStream }

export function emptyAgentStreamProgress(): AgentStreamProgress {
	return { consumed: 0, stream: { answer: '', reasoning: '' } }
}

/**
 * Any of these means a tool call is beginning, and which one arrives depends on
 * the provider: the SSE parsers announce `tool_call`, while Bedrock streams only
 * the arguments and the worker follows with `tool_execution`. Resetting on all
 * three is idempotent and keeps the rule provider-independent.
 */
const TOOL_TURN_STARTED = ['tool_call', 'tool_call_arguments', 'tool_execution']

const STREAM_EVENT_TYPES = [
	'token_delta',
	'reasoning_token_delta',
	'tool_call',
	'tool_call_arguments',
	'tool_execution',
	'tool_result'
]

function parseStreamEvent(line: string): (Record<string, unknown> & { type: string }) | undefined {
	let event: unknown
	try {
		event = JSON.parse(line)
	} catch {
		return undefined
	}
	if (!isRecord(event) || typeof event.type !== 'string') {
		return undefined
	}
	return STREAM_EVENT_TYPES.includes(event.type)
		? (event as Record<string, unknown> & { type: string })
		: undefined
}

/**
 * Whether `result_stream` is an agent's event stream rather than something a
 * script printed. Reads only the first complete line, because it runs on every
 * poll of a running job.
 */
export function isAgentStream(raw: string): boolean {
	let start = 0
	while (start < raw.length) {
		const end = raw.indexOf('\n', start)
		if (end === -1) {
			// Only a partial first line so far; wait for the poll that completes it.
			return false
		}
		const line = raw.slice(start, end)
		if (line.trim() !== '') {
			return parseStreamEvent(line) !== undefined
		}
		start = end + 1
	}
	return false
}

/**
 * Fold the events that arrived since `previous` into the answer so far.
 *
 * Incremental rather than a parse of the whole buffer: the stream only ever
 * grows, a poll can arrive every 50ms, and a `tool_result` event carries the
 * tool's entire output — so re-reading everything each time is quadratic in the
 * number of events with a large constant.
 */
export function advanceAgentStream(
	raw: string,
	previous: AgentStreamProgress
): AgentStreamProgress {
	// A trailing line with no newline yet is still being written, so it stays
	// unconsumed until the poll that completes it.
	const complete = raw.lastIndexOf('\n') + 1
	if (complete <= previous.consumed) {
		return previous
	}
	const stream: AgentStream = { ...previous.stream }
	for (const line of raw.slice(previous.consumed, complete).split('\n')) {
		if (line.trim() === '') {
			continue
		}
		const event = parseStreamEvent(line)
		if (!event) {
			continue
		}
		if (event.type === 'token_delta' && typeof event.content === 'string') {
			stream.answer += event.content
		} else if (event.type === 'reasoning_token_delta' && typeof event.content === 'string') {
			stream.reasoning += event.content
		} else if (typeof event.function_name === 'string') {
			if (TOOL_TURN_STARTED.includes(event.type)) {
				// A model can narrate and request a tool in the same turn, and the loop
				// then runs again. That narration is not part of the answer — the
				// finished result keeps the text of the last turn that produced any — so
				// a starting call resets rather than appending to what came before.
				stream.answer = ''
				stream.reasoning = ''
			}
			stream.tool = {
				name: event.function_name,
				running: event.type !== 'tool_result',
				success: event.type === 'tool_result' ? event.success === true : undefined
			}
		}
	}
	return { consumed: complete, stream }
}

/**
 * Token counts run to five and six figures, where the exact digit is noise. The
 * millions branch is not decoration: usage accumulates over every loop
 * iteration, and each one re-sends the whole context.
 */
export function formatTokenCount(count: number): string {
	if (count < 1000) {
		return String(count)
	}
	const [scaled, unit] = count < 1_000_000 ? [count / 1000, 'k'] : [count / 1_000_000, 'M']
	return `${scaled < 10 ? scaled.toFixed(1) : Math.round(scaled)}${unit}`
}
