import { WindmillChatApi } from './api'
import { followJob } from './follow'
import type { AgentStreamEvent } from './stream'
import type { ChatOptions } from './types'
import { errorResultMessage, extractChatAnswer, isErrorResult } from './utils'

/**
 * `run` and `follow` over a raw app's backend runnable, from the app's generated `wmill.ts`:
 * `rawAppRunnable(backendAsync.chat, streamJob)`. The runnable takes `user_message` and
 * `conversation_id` (plus any `inputs`) and returns `streamChatTurn(...)`. It runs as the
 * app, so a sandboxed app needs no frontend SDK scope, and its viewers no access to the agent.
 */
export function rawAppRunnable(
  // The generated signature is the runnable's own, whatever inputs it declares.
  start: (args: any) => Promise<string>,
  streamJob: (jobId: string, onUpdate?: (update: { new_result_stream?: string }) => void) => Promise<unknown>
): Required<Pick<ChatOptions, 'run' | 'follow'>> {
  return {
    run: async (args, { conversationId }) => String(await start({ ...args, conversation_id: conversationId })),
    follow: async (jobId, onStream) => {
      const result = await streamJob(jobId, (update) => {
        if (update.new_result_stream) onStream(update.new_result_stream)
      })
      // A runnable that returned a stream has that stream as its result: the answer already streamed.
      return typeof result === 'string' ? undefined : result
    }
  }
}

/**
 * For a raw app's backend runnable (TypeScript, Bun): runs one turn of a saved agent
 * (`agentPath`, an `ai_agent` resource) or of a chat-mode flow (`flowPath`) with the job's
 * own credential, its memory keyed by `conversationId`, and yields the answer for the
 * runnable to return as its result stream. A failed turn throws, failing the runnable.
 *
 * The runnable waits on that run, so it holds a worker for the whole turn: the run needs
 * another worker free, and an instance with a single worker stalls.
 */
export async function* streamChatTurn(
  target: { agentPath: string } | { flowPath: string },
  args: Record<string, unknown> & { user_message: string },
  conversationId: string
): AsyncGenerator<string> {
  const env = (globalThis as { process?: { env: Record<string, string | undefined> } }).process?.env ?? {}
  const api = new WindmillChatApi({
    baseUrl: env.BASE_INTERNAL_URL ?? env.BASE_URL ?? '',
    workspace: env.WM_WORKSPACE ?? '',
    token: env.WM_TOKEN
  })
  const options = { memoryId: conversationId }
  const jobId =
    'agentPath' in target
      ? await api.runAgent(target.agentPath, args, options)
      : await api.runFlow(target.flowPath, args, options)
  let streamedText = false
  for await (const event of followJob(api, jobId)) {
    if (event.type === 'stream') {
      if (event.events.length === 0) continue
      streamedText ||= event.events.some((e) => e.type === 'token_delta')
      yield event.events.map(streamLine).join('')
      continue
    }
    if (isErrorResult(event.result) && (await api.getCompletedResult(jobId)).success === false) {
      throw new Error(errorResultMessage(event.result))
    }
    // A structured answer, or a provider that does not stream, gives the text in the result only.
    const answer = streamedText ? undefined : extractChatAnswer(event.result)
    if (answer) yield streamLine({ type: 'token_delta', content: answer })
  }
}

/**
 * One NDJSON line that survives the worker's relay of a stream, which reads every `\n`
 * escape back as a line break: a newline is written `\u000a`, and a backslash `\`
 * so that a backslash before an `n` is never read as one either.
 */
export function streamLine(event: AgentStreamEvent): string {
  return (
    JSON.stringify(event).replace(/\\(u[0-9a-fA-F]{4}|.)/g, (escape, rest: string) =>
      rest === 'n' ? '\\u000a' : rest === '\\' ? '\\u005c' : escape
    ) + '\n'
  )
}
