import { WindmillApiError, type WindmillChatApi } from './api'
import { createStreamEventParser, type AgentStreamEvent } from './stream'
import { abortError, isAbortError, sleep } from './utils'

const RECONNECT_DELAY_MS = 300
const MAX_RECONNECT_DELAY_MS = 5000
/** Consecutive failed connections before the job is polled instead. */
const MAX_CONNECTION_FAILURES = 3
const RESULT_POLL_MS = 2000

export type FollowEvent =
  /** Agent events decoded from the job's result stream; empty when a chunk ended mid-line. */
  | { type: 'stream'; events: AgentStreamEvent[] }
  /** `streamLost`: the result was polled after the stream failed, so what streamed may stop short. */
  | { type: 'completed'; result: unknown; streamLost?: boolean }

/**
 * Follows a job to completion across the server's stream timeouts: every
 * connection resumes from the last `stream_offset`, so no delta is repeated and
 * the flow is never re-run. `onOffset` reports each offset, and its loss, so a
 * caller can resume later from another connection (see the AI SDK transport).
 *
 * The offset indexes the stream of one sub-job (`flow_stream_job_id`, the flow's
 * streaming step). A retried step gets a new one, so when the id changes the
 * offset is dropped and the connection reopened from that sub-job's start.
 *
 * A connection that fails (a proxy restarting, the network dropping) is retried with
 * backoff. The run is still going, so after a few failures in a row the job's result is
 * polled instead: the rest of the answer is not streamed, but the turn still ends.
 */
export async function* followJob(
  api: WindmillChatApi,
  jobId: string,
  options: { signal?: AbortSignal; streamOffset?: number; onOffset?: (offset: number | undefined) => void } = {}
): AsyncGenerator<FollowEvent> {
  let parser = createStreamEventParser()
  let offset = options.streamOffset
  let streamJobId: string | undefined
  let failures = 0
  while (failures < MAX_CONNECTION_FAILURES) {
    let reopen = false
    try {
      for await (const update of api.streamJob(jobId, { streamOffset: offset, signal: options.signal })) {
        // A ping proves the connection opened, not that it carries the job: only an update
        // clears the count, or a connection that pings and drops would never reach polling.
        if (update.type === 'ping') continue
        failures = 0
        if (update.type === 'timeout') {
          reopen = true
          break
        }
        if (update.type === 'error') throw new Error(update.error)
        if (update.type === 'notfound') throw new Error(`Job ${jobId} not found`)
        if (update.flow_stream_job_id && update.flow_stream_job_id !== streamJobId) {
          const switched = streamJobId !== undefined && offset !== undefined
          streamJobId = update.flow_stream_job_id
          if (switched) {
            // This connection skipped the new sub-job's first chunks: start it over.
            offset = undefined
            options.onOffset?.(undefined)
            parser = createStreamEventParser()
            reopen = true
            break
          }
        }
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
    } catch (e) {
      if (options.signal?.aborted || isAbortError(e) || !isConnectionFailure(e)) throw e
      failures++
      if (failures >= MAX_CONNECTION_FAILURES) break
      await sleep(Math.min(RECONNECT_DELAY_MS * 2 ** failures, MAX_RECONNECT_DELAY_MS), options.signal)
      continue
    }
    if (options.signal?.aborted) throw abortError()
    // The server closes the connection after its timeout; a connection that ends without
    // that event, and without the job completing, carried nothing to its end. The offset
    // lets the next one resume, and it counts like a failed one so a gateway closing every
    // stream this way still reaches the polling below rather than reconnecting for ever.
    if (!reopen) {
      failures++
      if (failures >= MAX_CONNECTION_FAILURES) break
      await sleep(RECONNECT_DELAY_MS, options.signal)
    }
  }
  while (true) {
    await sleep(RESULT_POLL_MS, options.signal)
    try {
      const { completed, result } = await api.getCompletedResult(jobId, options.signal)
      if (completed) {
        yield { type: 'completed', result, streamLost: true }
        return
      }
    } catch (e) {
      if (options.signal?.aborted || isAbortError(e) || !isConnectionFailure(e)) throw e
    }
  }
}

/**
 * A failure that says nothing about the job: the request never reached Windmill, or a
 * gateway in front of it answered. A 4xx from Windmill itself (not found, refused) does.
 */
function isConnectionFailure(e: unknown): boolean {
  if (e instanceof WindmillApiError) return e.status >= 500 || e.status === 0
  return e instanceof TypeError
}
