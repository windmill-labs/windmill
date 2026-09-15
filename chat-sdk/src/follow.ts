import type { WindmillChatApi } from './api'
import { createStreamEventParser, type AgentStreamEvent } from './stream'
import { abortError, sleep } from './utils'

const RECONNECT_DELAY_MS = 300

export type FollowEvent =
  /** Agent events decoded from the job's result stream; empty when a chunk ended mid-line. */
  | { type: 'stream'; events: AgentStreamEvent[] }
  | { type: 'completed'; result: unknown }

/**
 * Follows a job to completion across the server's stream timeouts: every
 * connection resumes from the last `stream_offset`, so no delta is repeated and
 * the flow is never re-run. `onOffset` reports each offset so a caller can
 * resume later from another connection (see the AI SDK transport).
 */
export async function* followJob(
  api: WindmillChatApi,
  jobId: string,
  options: { signal?: AbortSignal; streamOffset?: number; onOffset?: (offset: number) => void } = {}
): AsyncGenerator<FollowEvent> {
  const parser = createStreamEventParser()
  let offset = options.streamOffset
  while (true) {
    let timedOut = false
    for await (const update of api.streamJob(jobId, { streamOffset: offset, signal: options.signal })) {
      if (update.type === 'ping') continue
      if (update.type === 'timeout') {
        timedOut = true
        break
      }
      if (update.type === 'error') throw new Error(update.error)
      if (update.type === 'notfound') throw new Error(`Job ${jobId} not found`)
      if (update.stream_offset !== undefined) {
        offset = update.stream_offset
        options.onOffset?.(offset)
      }
      if (update.new_result_stream) {
        yield { type: 'stream', events: parser.push(update.new_result_stream) }
      }
      if (update.completed) {
        const rest = parser.flush()
        if (rest.length > 0) yield { type: 'stream', events: rest }
        yield { type: 'completed', result: update.only_result }
        return
      }
    }
    if (options.signal?.aborted) throw abortError()
    // The server closes the connection after its timeout; a dropped connection looks
    // the same minus the event. Either way the offset lets the next one resume.
    if (!timedOut) await sleep(RECONNECT_DELAY_MS, options.signal)
  }
}
