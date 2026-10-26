import { turnRunningError } from './api'
import type { ChatOptions } from './types'

/**
 * `run` and `follow` over a raw app's agent runnable (a path runnable of type `agent`, naming
 * a saved AI agent), behind the `runnable` option. `memory_id` is the conversation id, the
 * agent's memory key.
 */
export function rawAppRunnable(
  // The generated signature is the runnable's own.
  start: (args: any) => Promise<string>,
  streamJob: (jobId: string, onUpdate?: (update: { new_result_stream?: string }) => void) => Promise<unknown>
): Required<Pick<ChatOptions, 'run' | 'follow'>> {
  return {
    run: async (args, { conversationId }) => {
      try {
        return String(await start({ ...args, memory_id: conversationId }))
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
