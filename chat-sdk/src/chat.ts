import {
  WindmillApiError,
  WindmillChatApi,
  type FlowConversation,
  type FlowConversationMessage
} from './api'
import { resolveConfig, type ResolvedConfig } from './config'
import { createLocalHistory, type LocalHistory } from './history'
import { createStreamEventParser, type AgentStreamEvent } from './stream'
import type {
  Chat,
  ChatMessage,
  ChatOptions,
  ChatState,
  Conversation,
  ToolInvocation
} from './types'
import {
  conversationTitle,
  errorResultMessage,
  extractChatAnswer,
  isAbortError,
  isErrorResult,
  now,
  randomId,
  sleep
} from './utils'

const POLL_INTERVAL_MS = 1000
const RECONNECT_DELAY_MS = 300
/** Messages persist from spawned tasks that can land just after the flow completes. */
const RECONCILE_ATTEMPTS = 3
const RECONCILE_DELAY_MS = 400

interface Turn {
  controller: AbortController
  conversationId: string
  jobId?: string
  /** Id of the streaming assistant message; cleared when a tool call ends the round. */
  assistantId?: string
  streamedText: boolean
}

export function createChat(options: ChatOptions): Chat {
  return new ChatImpl(options)
}

class ChatImpl implements Chat {
  readonly #config: ResolvedConfig
  readonly #api: WindmillChatApi
  readonly #local: LocalHistory
  readonly #listeners = new Set<(state: ChatState) => void>()
  #state: ChatState
  #turn: Turn | undefined
  #page = 1

  constructor(options: ChatOptions) {
    this.#config = resolveConfig(options)
    this.#api = new WindmillChatApi({
      baseUrl: this.#config.baseUrl,
      workspace: this.#config.workspace,
      token: this.#config.token,
      fetch: this.#config.fetch
    })
    this.#local = createLocalHistory(
      this.#config.storage,
      `windmill-chat:${this.#config.baseUrl}:${this.#config.workspace}:${this.#config.flowPath}`
    )
    this.#state = {
      conversationId: undefined,
      messages: [],
      status: 'idle',
      error: undefined,
      conversations: [],
      history: this.#config.history,
      loadingMessages: false,
      hasMoreMessages: false
    }
  }

  getState = (): ChatState => this.#state

  subscribe = (listener: (state: ChatState) => void): (() => void) => {
    this.#listeners.add(listener)
    listener(this.#state)
    return () => {
      this.#listeners.delete(listener)
    }
  }

  sendMessage = async (
    text: string,
    options: { inputs?: Record<string, unknown> } = {}
  ): Promise<void> => {
    const content = text.trim()
    if (!content) return
    if (this.#turn) {
      throw new Error('windmill-chat: a message is already being answered; call stop() first')
    }
    const isNew = this.#state.conversationId === undefined
    const conversationId = this.#state.conversationId ?? randomId()
    const turn: Turn = { controller: new AbortController(), conversationId, streamedText: false }
    this.#turn = turn

    const timestamp = now()
    const conversation: Conversation = this.#state.conversations.find(
      (c) => c.id === conversationId
    ) ?? { id: conversationId, title: conversationTitle(content), createdAt: timestamp, updatedAt: timestamp }
    const touched = { ...conversation, updatedAt: timestamp }
    this.#set({
      conversationId,
      conversations: [touched, ...this.#state.conversations.filter((c) => c.id !== conversationId)],
      messages: [
        ...this.#state.messages,
        { id: `pending-${randomId()}`, role: 'user', content, success: true, createdAt: timestamp, pending: true }
      ],
      status: 'submitted',
      error: undefined
    })
    if (this.#state.history === 'local') this.#local.upsertConversation(touched)

    try {
      turn.jobId = await this.#api.runFlow(
        this.#config.flowPath,
        { ...this.#config.inputs, ...options.inputs, user_message: content },
        { memoryId: conversationId, signal: turn.controller.signal }
      )
      const stopPolling = this.#state.history === 'server' ? this.#startPolling(turn) : () => {}
      let result: unknown
      try {
        result = await this.#follow(turn, stopPolling)
      } finally {
        stopPolling()
      }
      await this.#finishTurn(turn, result, isNew)
    } catch (e) {
      // stop() and a conversation switch abort the turn and settle the state themselves.
      if (turn.controller.signal.aborted || isAbortError(e)) return
      this.#failTurn(turn, e)
    } finally {
      if (this.#turn === turn) this.#turn = undefined
    }
  }

  stop = async (): Promise<void> => {
    const turn = this.#turn
    if (!turn) return
    this.#detachTurn()
    if (this.#state.conversationId === turn.conversationId) {
      this.#set({ messages: finalized(this.#state.messages), status: 'idle' })
      this.#persistLocal()
    }
    if (turn.jobId) {
      // Needs `jobs:write` on a token; the stream is closed either way.
      await this.#api.cancelJob(turn.jobId).catch(() => {})
    }
    if (this.#state.history === 'server' && this.#state.conversationId === turn.conversationId) {
      // A cancelled flow persists its failure as the assistant's answer.
      await sleep(RECONCILE_DELAY_MS).catch(() => {})
      await this.#syncFromServer(turn.conversationId, { dropPending: false }).catch(() => {})
    }
  }

  newConversation = (): void => {
    this.#detachTurn()
    this.#page = 1
    this.#set({
      conversationId: undefined,
      messages: [],
      status: 'idle',
      error: undefined,
      loadingMessages: false,
      hasMoreMessages: false
    })
  }

  selectConversation = async (conversationId: string): Promise<void> => {
    if (conversationId === this.#state.conversationId) return
    this.#detachTurn()
    this.#page = 1
    this.#set({
      conversationId,
      messages: [],
      status: 'idle',
      error: undefined,
      loadingMessages: true,
      hasMoreMessages: false
    })
    if (this.#state.history !== 'server') {
      this.#set({
        messages: this.#state.history === 'local' ? this.#local.getMessages(conversationId) : [],
        loadingMessages: false
      })
      return
    }
    try {
      const rows = await this.#api.listMessages(conversationId, {
        perPage: this.#config.pageSize
      })
      if (this.#state.conversationId !== conversationId) return
      this.#set({
        messages: rows.map(fromRow),
        loadingMessages: false,
        hasMoreMessages: rows.length === this.#config.pageSize
      })
    } catch (e) {
      if (this.#state.conversationId !== conversationId) return
      if (this.#fallBackToLocal(e)) {
        this.#set({ messages: this.#local.getMessages(conversationId), loadingMessages: false })
        return
      }
      this.#set({ loadingMessages: false, status: 'error', error: toError(e) })
    }
  }

  loadConversations = async (
    options: { page?: number; perPage?: number } = {}
  ): Promise<Conversation[]> => {
    const page = options.page ?? 1
    let conversations: Conversation[]
    if (this.#state.history === 'server') {
      try {
        const rows = await this.#api.listConversations(this.#config.flowPath, {
          page,
          perPage: options.perPage ?? this.#config.pageSize
        })
        conversations = rows.map(fromConversation)
      } catch (e) {
        if (!this.#fallBackToLocal(e)) throw e
        conversations = this.#local.listConversations()
      }
    } else {
      conversations = this.#state.history === 'local' ? this.#local.listConversations() : []
    }
    const known = new Set(conversations.map((c) => c.id))
    this.#set({
      conversations:
        page === 1
          ? conversations
          : [...this.#state.conversations, ...conversations.filter((c) => !known.has(c.id))]
    })
    return conversations
  }

  deleteConversation = async (conversationId: string): Promise<void> => {
    if (this.#state.history === 'server') {
      await this.#api.deleteConversation(conversationId)
    } else if (this.#state.history === 'local') {
      this.#local.deleteConversation(conversationId)
    }
    this.#set({ conversations: this.#state.conversations.filter((c) => c.id !== conversationId) })
    if (this.#state.conversationId === conversationId) this.newConversation()
  }

  loadOlderMessages = async (): Promise<void> => {
    const conversationId = this.#state.conversationId
    if (
      !conversationId ||
      this.#state.history !== 'server' ||
      !this.#state.hasMoreMessages ||
      this.#state.loadingMessages
    ) {
      return
    }
    const page = this.#page + 1
    this.#set({ loadingMessages: true })
    try {
      const rows = await this.#api.listMessages(conversationId, {
        page,
        perPage: this.#config.pageSize
      })
      if (this.#state.conversationId !== conversationId) return
      const known = new Set(this.#state.messages.map((m) => m.id))
      this.#page = page
      this.#set({
        messages: [...rows.map(fromRow).filter((m) => !known.has(m.id)), ...this.#state.messages],
        hasMoreMessages: rows.length === this.#config.pageSize
      })
    } finally {
      if (this.#state.conversationId === conversationId) this.#set({ loadingMessages: false })
    }
  }

  destroy = (): void => {
    this.#detachTurn()
  }

  // ---- turn internals ----

  /** Follows the job's updates to completion, resuming across server-side stream timeouts. */
  async #follow(turn: Turn, onStreamStart: () => void): Promise<unknown> {
    const { signal } = turn.controller
    const parser = createStreamEventParser()
    let offset: number | undefined
    let started = false
    while (true) {
      let timedOut = false
      for await (const event of this.#api.streamJob(turn.jobId!, { streamOffset: offset, signal })) {
        if (event.type === 'ping') continue
        if (event.type === 'timeout') {
          timedOut = true
          break
        }
        if (event.type === 'error') throw new Error(event.error)
        if (event.type === 'notfound') throw new Error(`Job ${turn.jobId} not found`)
        if (event.stream_offset !== undefined) offset = event.stream_offset
        if (event.new_result_stream) {
          if (!started) {
            started = true
            // Persisted rows for the streaming step would duplicate what is streaming.
            onStreamStart()
          }
          this.#applyEvents(turn, parser.push(event.new_result_stream))
        }
        if (event.completed) {
          this.#applyEvents(turn, parser.flush())
          return event.only_result
        }
      }
      if (signal.aborted) throw new DOMException('The operation was aborted', 'AbortError')
      // The server closes the connection after its timeout; a dropped connection looks
      // the same minus the event. Either way the offset lets the next one resume.
      if (!timedOut) await sleep(RECONNECT_DELAY_MS, signal)
    }
  }

  #applyEvents(turn: Turn, events: AgentStreamEvent[]): void {
    if (events.length === 0 || !this.#turnActive(turn)) return
    let messages = [...this.#state.messages]
    const upsertTool = (
      callId: string,
      name: string,
      patch: Partial<ToolInvocation> & { content?: string; success?: boolean }
    ) => {
      const { content, success, ...toolPatch } = patch
      // Only this turn's tool messages are pending; a provider may reuse call ids across turns.
      const i = messages.findIndex(
        (m) => m.pending && m.role === 'tool' && m.tool?.callId === callId
      )
      if (i >= 0) {
        const existing = messages[i]
        messages[i] = {
          ...existing,
          content: content ?? existing.content,
          success: success ?? existing.success,
          tool: { ...existing.tool!, ...toolPatch }
        }
      } else {
        messages.push({
          id: `pending-${randomId()}`,
          role: 'tool',
          content: content ?? '',
          success: success ?? true,
          createdAt: now(),
          pending: true,
          tool: { callId, name, status: 'running', ...toolPatch }
        })
      }
    }
    const appendAssistant = (text: string, reasoning: string) => {
      const i = turn.assistantId
        ? messages.findIndex((m) => m.id === turn.assistantId)
        : -1
      if (i >= 0) {
        const m = messages[i]
        messages[i] = {
          ...m,
          content: m.content + text,
          reasoning: reasoning ? (m.reasoning ?? '') + reasoning : m.reasoning
        }
      } else {
        turn.assistantId = `pending-${randomId()}`
        messages.push({
          id: turn.assistantId,
          role: 'assistant',
          content: text,
          reasoning: reasoning || undefined,
          success: true,
          createdAt: now(),
          pending: true
        })
      }
    }
    for (const event of events) {
      switch (event.type) {
        case 'token_delta':
          turn.streamedText = true
          appendAssistant(event.content, '')
          break
        case 'reasoning_token_delta':
          appendAssistant('', event.content)
          break
        case 'tool_call':
          // The round's text is complete; text after the tool result is a new message.
          turn.assistantId = undefined
          upsertTool(event.call_id, event.function_name, { status: 'running' })
          break
        case 'tool_call_arguments':
          turn.assistantId = undefined
          upsertTool(event.call_id, event.function_name, { arguments: event.arguments })
          break
        case 'tool_execution':
          turn.assistantId = undefined
          upsertTool(event.call_id, event.function_name, { status: 'running' })
          break
        case 'tool_result':
          upsertTool(event.call_id, event.function_name, {
            status: event.success ? 'success' : 'error',
            result: event.result,
            success: event.success,
            // The same text Windmill persists for the tool message.
            content: event.success
              ? `Used ${event.function_name} tool`
              : `Error executing ${event.function_name}`
          })
          break
      }
    }
    this.#set({ messages, status: 'streaming' })
  }

  async #finishTurn(turn: Turn, result: unknown, isNew: boolean): Promise<void> {
    if (!this.#turnActive(turn)) return
    if (this.#state.history === 'server') {
      await this.#reconcileTurn(turn)
      if (!this.#turnActive(turn)) return
      this.#set({ status: 'idle' })
      if (isNew) await this.loadConversations().catch(() => {})
      return
    }
    let messages = this.#state.messages
    if (isErrorResult(result)) {
      // The envelope is also a legitimate result shape; the job's own status decides.
      const failed = await this.#api
        .getCompletedResult(turn.jobId!, turn.controller.signal)
        .then((r) => r.success === false)
        .catch(() => true)
      if (!this.#turnActive(turn)) return
      if (failed) {
        messages = [...messages, assistantMessage(errorResultMessage(result), false, turn.jobId)]
      }
    } else if (!turn.streamedText) {
      const answer = extractChatAnswer(result)
      if (answer !== undefined) {
        messages = [...messages, assistantMessage(answer, true, turn.jobId)]
      }
    }
    this.#set({ messages: finalized(messages), status: 'idle' })
    this.#persistLocal()
  }

  /**
   * Replaces the turn's optimistic messages with what the server persisted for it.
   * The answer's rows are written by the worker in their own transactions and may
   * trail the flow's completion, so an answer that hasn't landed yet is polled for.
   */
  async #reconcileTurn(turn: Turn): Promise<void> {
    for (let attempt = 1; attempt <= RECONCILE_ATTEMPTS; attempt++) {
      let rows: FlowConversationMessage[]
      try {
        rows = await this.#api.listMessages(turn.conversationId, {
          afterSeq: this.#lastSeq(),
          perPage: 100,
          signal: turn.controller.signal
        })
      } catch (e) {
        if (isAbortError(e)) throw e
        if (this.#fallBackToLocal(e)) break
        // Left as streamed; the next load of the conversation shows the server's copy.
        if (attempt === RECONCILE_ATTEMPTS) break
        await sleep(RECONCILE_DELAY_MS, turn.controller.signal)
        continue
      }
      if (!this.#turnActive(turn)) return
      const answered = rows.some((r) => r.message_type !== 'user')
      if (answered || attempt === RECONCILE_ATTEMPTS) {
        this.#mergeRows(rows, { dropPending: answered })
        break
      }
      this.#mergeRows(rows, { dropPending: false })
      await sleep(RECONCILE_DELAY_MS, turn.controller.signal)
    }
    if (!this.#turnActive(turn)) return
    this.#set({ messages: finalized(this.#state.messages) })
    // Mirrors the history in case the server side becomes unreadable later.
    this.#persistLocal()
  }

  #failTurn(turn: Turn, e: unknown): void {
    if (!this.#turnActive(turn)) return
    const error = toError(e)
    this.#set({
      messages: [...finalized(this.#state.messages), assistantMessage(error.message, false, turn.jobId)],
      status: 'error',
      error
    })
    this.#persistLocal()
  }

  /** Before the answer streams, earlier steps may already have persisted messages. */
  #startPolling(turn: Turn): () => void {
    let stopped = false
    const { signal } = turn.controller
    const loop = async () => {
      while (!stopped) {
        try {
          await sleep(POLL_INTERVAL_MS, signal)
        } catch {
          return
        }
        if (stopped) return
        try {
          const rows = await this.#api.listMessages(turn.conversationId, {
            afterSeq: this.#lastSeq(),
            perPage: 100,
            signal
          })
          if (!stopped && this.#turnActive(turn)) this.#mergeRows(rows, { dropPending: false })
        } catch {
          // transient; the completion reconciliation catches up
        }
      }
    }
    void loop()
    return () => {
      stopped = true
    }
  }

  async #syncFromServer(
    conversationId: string,
    options: { dropPending: boolean }
  ): Promise<void> {
    const rows = await this.#api.listMessages(conversationId, {
      afterSeq: this.#lastSeq(),
      perPage: 100
    })
    if (this.#state.conversationId !== conversationId) return
    this.#mergeRows(rows, options)
    this.#set({ messages: finalized(this.#state.messages) })
  }

  /**
   * Folds persisted rows into the message list. A row standing for a client-side
   * message (same role and text) takes its place and keeps what only the stream
   * knew (reasoning, tool call details); other rows append in server order. With
   * `dropPending`, the server's copy of the turn replaces every optimistic message,
   * except an assistant message that only carried reasoning: the server keeps none.
   */
  #mergeRows(rows: FlowConversationMessage[], options: { dropPending: boolean }): void {
    if (rows.length === 0) return
    const fresh = rows.map(fromRow)
    let messages = [...this.#state.messages]
    const known = new Set(messages.map((m) => m.id))
    const matchIndex = (row: ChatMessage) =>
      messages.findIndex((m) => m.seq === undefined && m.role === row.role && m.content === row.content)
    const streamOnly = (m: ChatMessage, row: ChatMessage): ChatMessage => ({
      ...row,
      reasoning: m.reasoning ?? row.reasoning,
      tool: m.tool ? { ...m.tool, status: row.tool?.status ?? m.tool.status } : row.tool
    })
    if (options.dropPending) {
      const carried = new Map<string, ChatMessage>()
      for (const m of messages) {
        if (m.pending) carried.set(`${m.role}\n${m.content}`, m)
      }
      messages = messages.filter(
        (m) => !m.pending || (m.role === 'assistant' && !m.content && m.reasoning)
      )
      for (const row of fresh) {
        if (known.has(row.id)) continue
        known.add(row.id)
        const i = matchIndex(row)
        const from = i >= 0 ? messages[i] : carried.get(`${row.role}\n${row.content}`)
        const merged = from ? streamOnly(from, row) : row
        if (i >= 0) messages[i] = merged
        else messages.push(merged)
      }
    } else {
      for (const row of fresh) {
        if (known.has(row.id)) continue
        known.add(row.id)
        const i = matchIndex(row)
        if (i >= 0) messages[i] = streamOnly(messages[i], row)
        else messages.push(row)
      }
    }
    this.#set({ messages })
  }

  #lastSeq(): number | undefined {
    let last: number | undefined
    for (const m of this.#state.messages) {
      if (m.seq !== undefined && (last === undefined || m.seq > last)) last = m.seq
    }
    return last
  }

  #persistLocal(): void {
    const id = this.#state.conversationId
    if (this.#state.history !== 'local' || !id) return
    this.#local.saveMessages(id, this.#state.messages)
    const conversation = this.#state.conversations.find((c) => c.id === id)
    if (conversation) this.#local.upsertConversation(conversation)
  }

  /** Whether an unreadable server history should silently become local history. */
  #fallBackToLocal(e: unknown): boolean {
    if (this.#state.history !== 'server' || this.#config.historyExplicit) return false
    if (e instanceof WindmillApiError && (e.status === 401 || e.status === 403)) {
      this.#set({ history: 'local' })
      return true
    }
    return false
  }

  #turnActive(turn: Turn): boolean {
    return this.#turn === turn && this.#state.conversationId === turn.conversationId
  }

  /** Stops following the current answer; the flow itself keeps running. */
  #detachTurn(): void {
    const turn = this.#turn
    if (!turn) return
    this.#turn = undefined
    turn.controller.abort()
  }

  #set(patch: Partial<ChatState>): void {
    this.#state = { ...this.#state, ...patch }
    for (const listener of this.#listeners) listener(this.#state)
  }
}

function fromRow(row: FlowConversationMessage): ChatMessage {
  const toolName =
    row.message_type === 'tool'
      ? /^Used (.+) tool$/.exec(row.content)?.[1] ?? /^Error executing (.+)$/.exec(row.content)?.[1]
      : undefined
  const success = row.success ?? true
  return {
    id: row.id,
    role: row.message_type,
    content: row.content,
    success,
    createdAt: row.created_at,
    jobId: row.job_id ?? undefined,
    stepName: row.step_name ?? undefined,
    pending: false,
    seq: row.created_seq,
    tool: toolName ? { name: toolName, status: success ? 'success' : 'error' } : undefined
  }
}

function fromConversation(row: FlowConversation): Conversation {
  return {
    id: row.id,
    title: row.title ?? undefined,
    createdAt: row.created_at,
    updatedAt: row.updated_at
  }
}

function assistantMessage(content: string, success: boolean, jobId: string | undefined): ChatMessage {
  return {
    id: `local-${randomId()}`,
    role: 'assistant',
    content,
    success,
    createdAt: now(),
    jobId,
    pending: false
  }
}

function finalized(messages: ChatMessage[]): ChatMessage[] {
  return messages.some((m) => m.pending)
    ? messages.map((m) => (m.pending ? { ...m, pending: false } : m))
    : messages
}

function toError(e: unknown): Error {
  return e instanceof Error ? e : new Error(String(e))
}
