import { TurnRunningError, WindmillChatApi } from './api'
import { isInlineAttachment, uploadAttachments } from './attachments'
import { followJob } from './follow'
import type { AgentStreamEvent } from './stream'
import type { ChatOptions } from './types'
import { errorResultMessage, extractChatAnswer, isErrorResult, randomId } from './utils'

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
 * Attachments the chat sent (an `args` value of `data:` URL files) are stored in the
 * workspace's object storage first, and the run gets their references.
 *
 * A turn the chat stopped following goes on running, and its conversation takes no other
 * until it ends: the next one waits for it here. The runnable waits on its run, so it holds
 * a worker for the whole turn, and the run needs another worker free.
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
  args = { ...args }
  for (const [key, value] of Object.entries(args)) {
    const files = Array.isArray(value) ? value : [value]
    if (files.length === 0 || !files.every(isInlineAttachment)) continue
    const uploaded = await uploadAttachments(api, files, randomId())
    args[key] = Array.isArray(value) ? uploaded : uploaded[0]
  }
  let jobId: string
  while (true) {
    try {
      jobId =
        'agentPath' in target
          ? await api.runAgent(target.agentPath, args, options)
          : await api.runFlow(target.flowPath, args, options)
      break
    } catch (e) {
      if (!(e instanceof TurnRunningError)) throw e
      for await (const _ of followJob(api, e.turn.jobId));
    }
  }
  /** The text of the round streaming now: a tool call ends a round, and the answer is the last one. */
  let roundText = ''
  for await (const event of followJob(api, jobId)) {
    if (event.type === 'stream') {
      if (event.events.length === 0) continue
      for (const e of event.events) {
        if (e.type === 'token_delta') roundText += e.content
        else if (e.type !== 'reasoning_token_delta') roundText = ''
      }
      yield event.events.map(streamLine).join('')
      continue
    }
    if (isErrorResult(event.result) && (await api.getCompletedResult(jobId)).success === false) {
      throw new Error(errorResultMessage(event.result))
    }
    // A structured answer, or a provider that does not stream, gives the text in the result only;
    // so does the rest of an answer whose stream was lost. A stream lost before a tool call left
    // an earlier round's text, which the answer does not continue: it follows that text whole.
    const answer = extractChatAnswer(event.result)
    if (!answer || (roundText && !event.streamLost)) continue
    const rest = answer.startsWith(roundText) ? answer.slice(roundText.length) : `\n\n${answer}`
    if (rest) yield streamLine({ type: 'token_delta', content: rest })
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
