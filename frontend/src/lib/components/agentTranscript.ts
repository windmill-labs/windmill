import type { WebSearchSource } from './copilot/chat/shared'
import type { AgentMessage } from './aiAgentResult'

/**
 * One readable turn of an agent run. Built from the envelope alone: the tool
 * arguments come from the assistant message that asked for the call, and the
 * result from the `tool` message that answered it, so a transcript renders
 * without waiting on any request. A tool's child job is enrichment (logs,
 * duration, whether it succeeded), not what makes the row.
 */
export type TranscriptEntry =
	| { kind: 'system'; content: string }
	| { kind: 'user'; content: string }
	| { kind: 'assistant'; content: string; sources?: WebSearchSource[] }
	| { kind: 'search'; content: string; sources?: WebSearchSource[] }
	| {
			kind: 'tool'
			name: string
			args?: string
			result: string
			/** Present for a flow-module tool, which runs as its own job. */
			jobId?: string
			/** An MCP tool runs in the worker, so it names its server instead. */
			resourcePath?: string
	  }

/** `content` is a string for text messages and a part list once images are involved. */
function contentText(content: unknown): string {
	if (typeof content === 'string') {
		return content
	}
	if (Array.isArray(content)) {
		return content
			.map((part) =>
				part && typeof part === 'object' && typeof (part as { text?: unknown }).text === 'string'
					? (part as { text: string }).text
					: ''
			)
			.join('')
	}
	return ''
}

function sourcesOf(message: AgentMessage): WebSearchSource[] | undefined {
	const annotations = message.annotations
	if (!annotations?.length) {
		return undefined
	}
	const sources = annotations
		.filter((a) => typeof a?.url === 'string')
		.map((a) => ({ url: a.url, title: a.title }))
	return sources.length > 0 ? sources : undefined
}

export function buildTranscript(messages: AgentMessage[]): TranscriptEntry[] {
	// The arguments live on the assistant message that requested the call, while
	// the action tag and the result live on the `tool` message answering it, so
	// the two are joined by `tool_call_id`.
	const argsByCallId = new Map<string, string>()
	for (const message of messages) {
		for (const call of message.tool_calls ?? []) {
			if (call.id && typeof call.function?.arguments === 'string') {
				argsByCallId.set(call.id, call.function.arguments)
			}
		}
	}

	const entries: TranscriptEntry[] = []
	for (const message of messages) {
		const action = message.agent_action
		if (action?.type === 'tool_call') {
			entries.push({
				kind: 'tool',
				name: action.function_name,
				args: message.tool_call_id ? argsByCallId.get(message.tool_call_id) : undefined,
				result: contentText(message.content),
				jobId: action.job_id
			})
			continue
		}
		if (action?.type === 'mcp_tool_call') {
			entries.push({
				kind: 'tool',
				name: action.function_name,
				// An MCP call records its arguments on the action itself: it never
				// became a job, so there is nowhere else for them to live.
				args: action.arguments ? JSON.stringify(action.arguments, null, 2) : undefined,
				result: contentText(message.content),
				resourcePath: action.resource_path
			})
			continue
		}
		if (action?.type === 'web_search') {
			entries.push({
				kind: 'search',
				content: contentText(message.content),
				sources: sourcesOf(message)
			})
			continue
		}
		// Messages with no action are the conversation itself: the prompt, what the
		// user asked, and anything loaded back from memory.
		const content = contentText(message.content)
		if (content === '') {
			continue
		}
		if (message.role === 'system') {
			entries.push({ kind: 'system', content })
		} else if (message.role === 'user') {
			entries.push({ kind: 'user', content })
		} else if (message.role === 'assistant') {
			entries.push({ kind: 'assistant', content, sources: sourcesOf(message) })
		}
	}
	return entries
}

export function transcriptJobIds(entries: TranscriptEntry[]): string[] {
	return entries.flatMap((entry) => (entry.kind === 'tool' && entry.jobId ? [entry.jobId] : []))
}
