import { turnRunningError } from './api'
import type { ChatOptions } from './types'

/**
 * `run` and `follow` over a raw app's chat runnable (a saved AI agent, or a flow in chat mode),
 * behind the `runnable` option. The conversation id goes as `memoryId`, the run's `memory_id`
 * query parameter, which keys the conversation and its memory.
 */
export function rawAppRunnable(
  // The generated signature is the runnable's own.
  start: (args: any, opts: { memoryId: string }) => Promise<string>,
  streamJob: (jobId: string, onUpdate?: (update: { new_result_stream?: string }) => void) => Promise<unknown>
): Required<Pick<ChatOptions, 'run' | 'follow'>> {
  return {
    run: async (args, { conversationId }) => {
      try {
        return String(await start(args, { memoryId: conversationId }))
      } catch (e) {
        // The host relays the server's refusal as the error's text, possibly behind a stack.
        const text = e instanceof Error ? e.message : String(e)
        throw turnRunningError(text.slice(text.indexOf('{'), text.lastIndexOf('}') + 1)) ?? e
      }
    },
    follow: (jobId, onStream) =>
      streamJob(jobId, (update) => {
        if (update.new_result_stream) onStream(update.new_result_stream)
      })
  }
}
