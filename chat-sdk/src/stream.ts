/** The events an AI agent step streams, one JSON object per line of the job's result stream. */
export type AgentStreamEvent =
  | { type: 'token_delta'; content: string }
  | { type: 'reasoning_token_delta'; content: string }
  | { type: 'tool_call'; call_id: string; function_name: string }
  | { type: 'tool_call_arguments'; call_id: string; function_name: string; arguments: string }
  | { type: 'tool_execution'; call_id: string; function_name: string }
  | {
      type: 'tool_result'
      call_id: string
      function_name: string
      result: string
      success: boolean
    }

const KNOWN_TYPES = new Set([
  'token_delta',
  'reasoning_token_delta',
  'tool_call',
  'tool_call_arguments',
  'tool_execution',
  'tool_result'
])

/**
 * Incremental parser for the `new_result_stream` chunks of a job update. A chunk is
 * not guaranteed to end on a line boundary, so an incomplete last line waits for the
 * next `push` (or `flush` once the job completes).
 */
export function createStreamEventParser() {
  let pending = ''
  return {
    push(chunk: string): AgentStreamEvent[] {
      pending += chunk
      const lastNewline = pending.lastIndexOf('\n')
      if (lastNewline === -1) return []
      const complete = pending.slice(0, lastNewline)
      pending = pending.slice(lastNewline + 1)
      return parseStreamEvents(complete)
    },
    flush(): AgentStreamEvent[] {
      const rest = pending
      pending = ''
      return parseStreamEvents(rest)
    }
  }
}

/** Parses complete NDJSON lines; lines that aren't agent events are skipped. */
export function parseStreamEvents(ndjson: string): AgentStreamEvent[] {
  const events: AgentStreamEvent[] = []
  for (const line of ndjson.split('\n')) {
    const trimmed = line.trim()
    if (!trimmed) continue
    try {
      const parsed = JSON.parse(trimmed)
      if (parsed && typeof parsed === 'object' && KNOWN_TYPES.has(parsed.type)) {
        events.push(parsed as AgentStreamEvent)
      }
    } catch {
      // not an agent event
    }
  }
  return events
}
