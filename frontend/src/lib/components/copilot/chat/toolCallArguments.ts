import type { ChatCompletionMessageParam } from 'openai/resources/chat/completions.mjs'

/**
 * A tool call whose `arguments` string is not valid JSON (typically a stream
 * cut mid-arguments) must never reach a provider: the whole request is
 * rejected, and once persisted in the session history every follow-up request
 * fails the same way.
 *
 * Empty arguments count as valid: some providers stream '' for no-arg tool
 * calls, and flagging those would wrongly report the tool as not executed.
 * Callers persisting arguments must normalize '' to '{}' themselves.
 */
export function hasValidToolCallArguments(args: string | undefined): boolean {
	if (!args) {
		return true
	}
	try {
		JSON.parse(args)
		return true
	} catch {
		return false
	}
}

// '' is valid per hasValidToolCallArguments but JSON.parse('') still throws
// provider-side, so replaying history additionally requires non-empty args.
function isReplayableArguments(args: string | undefined): boolean {
	return !!args && hasValidToolCallArguments(args)
}

/**
 * Replaces unparseable or empty assistant tool_call arguments with '{}' in the
 * outgoing copy of the history, so a session that already persisted a
 * truncated tool call (before parseOpenAICompletion guarded against it)
 * recovers instead of failing every request. The paired tool result already
 * tells the model the call failed.
 */
export function sanitizeToolCallArguments(
	messages: ChatCompletionMessageParam[]
): ChatCompletionMessageParam[] {
	return messages.map((m) => {
		if (
			m.role !== 'assistant' ||
			!m.tool_calls?.some(
				(t) => t.type === 'function' && !isReplayableArguments(t.function.arguments)
			)
		) {
			return m
		}
		return {
			...m,
			tool_calls: m.tool_calls.map((t) =>
				t.type === 'function' && !isReplayableArguments(t.function.arguments)
					? { ...t, function: { ...t.function, arguments: '{}' } }
					: t
			)
		}
	})
}
