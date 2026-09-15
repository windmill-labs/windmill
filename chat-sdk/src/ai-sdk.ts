import type { ChatTransport, UIMessage, UIMessageChunk, UIMessagePart } from 'ai'
import { WindmillApiError, WindmillChatApi, type WindmillChatApiOptions } from './api'
import { followJob } from './follow'
import type { AgentStreamEvent } from './stream'
import type { ChatMessage, Conversation } from './types'
import {
  conversationIdFor,
  errorResultMessage,
  extractChatAnswer,
  isAbortError,
  isErrorResult,
  parseJsonOr,
  randomId
} from './utils'

export interface WindmillChatTransportOptions extends WindmillChatApiOptions {
  /** Path of a deployed flow with chat mode enabled, e.g. `f/support/assistant`. */
  flowPath: string
  /** Extra flow inputs sent with every message; `sendMessage(msg, { body })` adds per-message ones. */
  inputs?: Record<string, unknown>
}

export interface WindmillChatTransport<UI_MESSAGE extends UIMessage = UIMessage>
  extends ChatTransport<UI_MESSAGE> {
  /** The Windmill conversation id behind an AI SDK chat id (a UUID chat id is used as is). */
  conversationId(chatId: string): string
  /** Server history of a chat as `UIMessage`s, oldest first, for `useChat({ messages })`. Needs `flow_conversations:read`. */
  loadMessages(chatId: string, options?: { page?: number; perPage?: number }): Promise<UI_MESSAGE[]>
  /** The user's conversations for this flow, most recent first. */
  listConversations(options?: { page?: number; perPage?: number }): Promise<Conversation[]>
  deleteConversation(chatId: string): Promise<void>
}

interface JobEntry {
  jobId: string
  offset?: number
  done: boolean
}

/**
 * A Vercel AI SDK `ChatTransport` over a chat-mode flow: `useChat({ transport })`
 * (and AI Elements, which builds on it) then work against Windmill unchanged.
 * The chat id is the conversation, so a UUID id lines up with server history.
 */
export function createWindmillChatTransport<UI_MESSAGE extends UIMessage = UIMessage>(
  options: WindmillChatTransportOptions
): WindmillChatTransport<UI_MESSAGE> {
  const api = new WindmillChatApi(options)
  // One in-flight or finished job per chat, for `reconnectToStream`.
  const jobs = new Map<string, JobEntry>()

  return {
    conversationId: conversationIdFor,

    async sendMessages({ chatId, messages, abortSignal, body }) {
      const last = messages[messages.length - 1]
      if (!last || last.role !== 'user') {
        throw new Error('windmill-chat: the last message must be a user message')
      }
      if (last.parts.some((p) => p.type === 'file')) {
        throw new Error(
          'windmill-chat: attachments are not supported; upload the file yourself and pass its reference through `body`'
        )
      }
      const text = last.parts
        .filter((p): p is Extract<UIMessagePart<never, never>, { type: 'text' }> => p.type === 'text')
        .map((p) => p.text)
        .join('\n')
      const memoryId = conversationIdFor(chatId)
      const jobId = await api.runFlow(
        options.flowPath,
        { ...options.inputs, ...(body as Record<string, unknown> | undefined), user_message: text },
        { memoryId, signal: abortSignal }
      )
      const entry: JobEntry = { jobId, done: false }
      jobs.set(chatId, entry)
      return chunkStream(api, entry, abortSignal)
    },

    async reconnectToStream({ chatId, abortSignal }) {
      const entry = jobs.get(chatId)
      if (!entry || entry.done) return null
      return chunkStream(api, entry, abortSignal)
    },

    async loadMessages(chatId, pagination) {
      // A chat that hasn't sent anything yet has no conversation on the server.
      const rows = await api.listMessages(conversationIdFor(chatId), pagination).catch((e) => {
        if (e instanceof WindmillApiError && e.status === 404) return []
        throw e
      })
      return toUIMessages(
        rows.map((row) => ({
          id: row.id,
          serverId: row.id,
          role: row.message_type,
          content: row.content,
          success: row.success ?? true,
          createdAt: row.created_at,
          jobId: row.job_id ?? undefined,
          stepName: row.step_name ?? undefined,
          pending: false,
          seq: row.created_seq,
          tool: toolFromRowContent(row.message_type, row.content, row.success ?? true)
        }))
      ) as UI_MESSAGE[]
    },

    async listConversations(pagination) {
      const rows = await api.listConversations(options.flowPath, pagination)
      return rows.map((row) => ({
        id: row.id,
        title: row.title ?? undefined,
        createdAt: row.created_at,
        updatedAt: row.updated_at
      }))
    },

    async deleteConversation(chatId) {
      await api.deleteConversation(conversationIdFor(chatId))
      jobs.delete(chatId)
    }
  }
}

function toolFromRowContent(role: string, content: string, success: boolean): ChatMessage['tool'] {
  if (role !== 'tool') return undefined
  const name = /^Used (.+) tool$/.exec(content)?.[1] ?? /^Error executing (.+)$/.exec(content)?.[1]
  return name ? { name, status: success ? 'success' : 'error' } : undefined
}

/** Streams a job's answer as AI SDK chunks; resumes from `entry.offset` when the job is already running. */
function chunkStream(
  api: WindmillChatApi,
  entry: JobEntry,
  signal: AbortSignal | undefined
): ReadableStream<UIMessageChunk> {
  return new ReadableStream<UIMessageChunk>({
    async start(controller) {
      const parts = new PartWriter((chunk) => controller.enqueue(chunk))
      parts.emit({ type: 'start' })
      try {
        let failure: string | undefined
        for await (const event of followJob(api, entry.jobId, {
          signal,
          streamOffset: entry.offset,
          onOffset: (offset) => {
            entry.offset = offset
          }
        })) {
          if (event.type === 'stream') {
            for (const e of event.events) parts.apply(e)
            continue
          }
          parts.closeOpen()
          failure = await failureText(api, entry.jobId, event.result, signal)
          if (failure === undefined && !parts.streamedText) {
            // No agent streamed: the flow's result is the answer.
            const answer = extractChatAnswer(event.result)
            if (answer !== undefined) {
              parts.text(answer)
              parts.closeOpen()
            }
          }
        }
        entry.done = true
        parts.emit(failure === undefined ? { type: 'finish' } : { type: 'error', errorText: failure })
      } catch (e) {
        if (!isAbortError(e)) {
          parts.closeOpen()
          parts.emit({ type: 'error', errorText: e instanceof Error ? e.message : String(e) })
        }
      } finally {
        controller.close()
      }
    }
  })
}

/** A completed flow's error, when the job did fail (the envelope alone is a legitimate result). */
async function failureText(
  api: WindmillChatApi,
  jobId: string,
  result: unknown,
  signal: AbortSignal | undefined
): Promise<string | undefined> {
  if (!isErrorResult(result)) return undefined
  const failed = await api
    .getCompletedResult(jobId, signal)
    .then((r) => r.success === false)
    .catch(() => true)
  return failed ? errorResultMessage(result) : undefined
}

/**
 * Turns agent events into AI SDK chunks. Text and reasoning are open parts that a
 * tool call closes (a new round starts new parts); tool calls are `dynamic-tool`
 * parts, since the UI declares no tools of its own.
 */
class PartWriter {
  streamedText = false
  #textId: string | undefined
  #reasoningId: string | undefined
  #started = new Set<string>()
  #inputSent = new Set<string>()

  constructor(readonly emit: (chunk: UIMessageChunk) => void) {}

  text(delta: string): void {
    this.streamedText = true
    if (!this.#textId) {
      this.#textId = randomId()
      this.emit({ type: 'text-start', id: this.#textId })
    }
    this.emit({ type: 'text-delta', id: this.#textId, delta })
  }

  reasoning(delta: string): void {
    if (!this.#reasoningId) {
      this.#reasoningId = randomId()
      this.emit({ type: 'reasoning-start', id: this.#reasoningId })
    }
    this.emit({ type: 'reasoning-delta', id: this.#reasoningId, delta })
  }

  closeOpen(): void {
    if (this.#reasoningId) {
      this.emit({ type: 'reasoning-end', id: this.#reasoningId })
      this.#reasoningId = undefined
    }
    if (this.#textId) {
      this.emit({ type: 'text-end', id: this.#textId })
      this.#textId = undefined
    }
  }

  apply(event: AgentStreamEvent): void {
    switch (event.type) {
      case 'token_delta':
        this.text(event.content)
        break
      case 'reasoning_token_delta':
        this.reasoning(event.content)
        break
      case 'tool_call':
        this.closeOpen()
        this.#toolStart(event.call_id, event.function_name)
        break
      case 'tool_call_arguments':
        this.closeOpen()
        this.#toolStart(event.call_id, event.function_name)
        this.#inputSent.add(event.call_id)
        this.emit({
          type: 'tool-input-available',
          toolCallId: event.call_id,
          toolName: event.function_name,
          input: parseJsonOr(event.arguments),
          dynamic: true
        })
        break
      case 'tool_execution':
        this.closeOpen()
        this.#toolStart(event.call_id, event.function_name)
        break
      case 'tool_result':
        this.#toolStart(event.call_id, event.function_name)
        if (!this.#inputSent.has(event.call_id)) {
          this.#inputSent.add(event.call_id)
          this.emit({
            type: 'tool-input-available',
            toolCallId: event.call_id,
            toolName: event.function_name,
            input: undefined,
            dynamic: true
          })
        }
        this.emit(
          event.success
            ? { type: 'tool-output-available', toolCallId: event.call_id, output: parseJsonOr(event.result), dynamic: true }
            : { type: 'tool-output-error', toolCallId: event.call_id, errorText: event.result, dynamic: true }
        )
        break
    }
  }

  #toolStart(callId: string, name: string): void {
    if (this.#started.has(callId)) return
    this.#started.add(callId)
    this.emit({ type: 'tool-input-start', toolCallId: callId, toolName: name, dynamic: true })
  }
}

/**
 * `ChatMessage`s (Windmill's role-per-row model) as `UIMessage`s: an assistant
 * turn becomes one message whose parts carry its text, reasoning and tool calls.
 */
export function toUIMessages(messages: ChatMessage[]): UIMessage[] {
  const out: UIMessage[] = []
  for (const m of messages) {
    if (m.role === 'user' || m.role === 'system') {
      out.push({ id: m.id, role: m.role, parts: [{ type: 'text', text: m.content }] })
      continue
    }
    let target = out[out.length - 1]
    if (!target || target.role !== 'assistant') {
      target = { id: m.id, role: 'assistant', parts: [] }
      out.push(target)
    }
    if (m.role === 'tool') {
      const toolCallId = m.tool?.callId ?? m.id
      const toolName = m.tool?.name ?? 'tool'
      const input = parseJsonOr(m.tool?.arguments)
      target.parts.push(
        m.success
          ? { type: 'dynamic-tool', toolName, toolCallId, state: 'output-available', input, output: parseJsonOr(m.tool?.result) ?? m.content }
          : { type: 'dynamic-tool', toolName, toolCallId, state: 'output-error', input, errorText: m.tool?.result ?? m.content }
      )
      continue
    }
    if (m.reasoning) target.parts.push({ type: 'reasoning', text: m.reasoning, state: 'done' })
    if (m.content) target.parts.push({ type: 'text', text: m.content, state: 'done' })
  }
  return out
}
