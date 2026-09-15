/**
 * The AI agent's streamed events, as the worker writes them.
 *
 * One SSE chunk can carry several lines, so parsing returns a list: a chunk holding a
 * tool call and its result must not collapse to whichever came last. Mirrors
 * `StreamingEvent` in backend/windmill-ai/src/types.rs (tagged `type`, snake_case).
 */
export type StreamEvent =
	| { kind: 'token'; content: string }
	| { kind: 'reasoning'; content: string }
	| { kind: 'tool_call'; callId: string; name: string }
	| { kind: 'tool_arguments'; callId: string; name: string; arguments: string }
	| { kind: 'tool_execution'; callId: string; name: string }
	| { kind: 'tool_result'; callId: string; name: string; result: string; success: boolean }

export function parseStreamEvents(streamData: string): StreamEvent[] {
	const events: StreamEvent[] = []
	for (const line of streamData.trim().split('\n')) {
		if (!line.trim()) continue
		let parsed: any
		try {
			parsed = JSON.parse(line)
		} catch (e) {
			console.error('Failed to parse stream line:', line, e)
			continue
		}
		switch (parsed?.type) {
			case 'token_delta':
				if (parsed.content) events.push({ kind: 'token', content: parsed.content })
				break
			case 'reasoning_token_delta':
				if (parsed.content) events.push({ kind: 'reasoning', content: parsed.content })
				break
			case 'tool_call':
				events.push({ kind: 'tool_call', callId: parsed.call_id, name: parsed.function_name })
				break
			case 'tool_call_arguments':
				events.push({
					kind: 'tool_arguments',
					callId: parsed.call_id,
					name: parsed.function_name,
					arguments: parsed.arguments ?? ''
				})
				break
			case 'tool_execution':
				events.push({ kind: 'tool_execution', callId: parsed.call_id, name: parsed.function_name })
				break
			case 'tool_result':
				events.push({
					kind: 'tool_result',
					callId: parsed.call_id,
					name: parsed.function_name,
					result: parsed.result ?? '',
					success: parsed.success !== false
				})
				break
		}
	}
	return events
}

/** One-line summary of a tool call, for a surface with no room for the call itself. */
export function toolSummary(name: string, success: boolean): string {
	return success ? `Used ${name} tool` : `Failed to use ${name} tool`
}

/**
 * Flattened view of a chunk, for callers that render a single running string.
 * Keeps the shape AppChat has always consumed.
 */
export function parseStreamDeltas(streamData: string): {
	content: string
	type?: string
	success?: boolean
} {
	let content = ''
	let type = 'message'
	let success = true
	for (const event of parseStreamEvents(streamData)) {
		if (event.kind === 'token') {
			content += event.content
		} else if (event.kind === 'tool_result') {
			type = 'tool_result'
			success = event.success
			content = toolSummary(event.name, event.success)
		}
	}
	return { content, type, success }
}
