import {
  WindmillApiError,
  WindmillChatApi,
  type ConversationKind,
  type FlowConversation,
  type FlowConversationMessage
} from './api'
import { resolveConfig, type ResolvedConfig } from './config'
import { followJob } from './follow'
import { createLocalHistory, type LocalHistory } from './history'
import type { AgentStreamEvent } from './stream'
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
  truncateTitle,
  errorResultMessage,
  extractChatAnswer,
  isAbortError,
  isErrorResult,
  now,
  randomId,
  sleep
} from './utils'

const POLL_INTERVAL_MS = 1000
/** Local history mirrors the state; a stream of deltas is coalesced into one write. */
const PERSIST_DEBOUNCE_MS = 250
/** Messages persist from spawned tasks that can land just after the flow completes. */
const RECONCILE_ATTEMPTS = 3
const RECONCILE_DELAY_MS = 400

interface Turn {
  controller: AbortController
  conversationId: string
  /** Id of the turn's user message; the answer is whatever follows it. */
  userMessageId: string
  jobId?: string
  /** The flow job and its step jobs; a persisted answer carries one of them as `job_id`. */
  jobIds?: Set<string>
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
  /** The kind the caller last listed, so the refresh after a new turn lists the same rows. */
  #conversationKind: ConversationKind | undefined
  #persistTimer: ReturnType<typeof setTimeout> | undefined

  constructor(options: ChatOptions) {
    this.#config = resolveConfig(options)
    this.#api = new WindmillChatApi({
      baseUrl: this.#config.baseUrl,
      workspace: this.#config.workspace,
      token: this.#config.token,
      fetch: this.#config.fetch,
      pollDelayMs: this.#config.pollDelayMs
    })
    this.#local = createLocalHistory(
      this.#config.storage,
      `windmill-chat:${this.#config.baseUrl}:${this.#config.workspace}:${this.#config.flowPath}` +
        (this.#config.storageKey ? `:${this.#config.storageKey}` : '')
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
    const turn: Turn = {
      controller: new AbortController(),
      conversationId,
      userMessageId: `pending-${randomId()}`,
      streamedText: false
    }
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
        { id: turn.userMessageId, role: 'user', content, success: true, createdAt: timestamp, pending: true }
      ],
      status: 'submitted',
      error: undefined
    })
    this.#rememberConversation()

    try {
      const args = { ...this.#config.inputs, ...options.inputs, user_message: content }
      const context = { memoryId: conversationId, conversationId, signal: turn.controller.signal }
      turn.jobId = this.#config.run
        ? await this.#config.run(args, context)
        : await this.#api.runFlow(this.#config.flowPath, args, context)
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
    if (this.#state.history !== 'server') return
    // A cancelled flow persists its failure as the assistant's answer. Picked up only
    // while the conversation is still idle: a turn started meanwhile owns the state.
    await sleep(RECONCILE_DELAY_MS).catch(() => {})
    if (this.#turn || this.#state.conversationId !== turn.conversationId) return
    await this.#syncFromServer(turn.conversationId).catch(() => {})
  }

  newConversation = (): void => {
    this.#leaveConversation()
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
    this.#leaveConversation()
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
    options: { page?: number; perPage?: number; kind?: ConversationKind } = {}
  ): Promise<Conversation[]> => {
    const page = options.page ?? 1
    // A different kind is a different listing: its first rows replace the held ones, on
    // whichever page they were asked for.
    const kindChanged = 'kind' in options && options.kind !== this.#conversationKind
    if ('kind' in options) this.#conversationKind = options.kind
    const kind = this.#conversationKind
    let conversations: Conversation[]
    if (this.#state.history === 'server') {
      try {
        const rows = await this.#api.listConversations(this.#config.flowPath, {
          page,
          perPage: options.perPage ?? this.#config.pageSize,
          kind
        })
        conversations = rows.map(fromConversation)
      } catch (e) {
        if (!this.#fallBackToLocal(e)) throw e
        conversations = this.#local.listConversations()
      }
    } else {
      conversations = this.#state.history === 'local' ? this.#local.listConversations() : []
    }
    // Another kind was asked for while this list was on its way: its rows are not the
    // listing any more, whichever response lands last.
    if (kind !== this.#conversationKind) return conversations
    const known = new Set(this.#state.conversations.map((c) => c.id))
    this.#set({
      conversations:
        page === 1 || kindChanged
          ? conversations
          : [...this.#state.conversations, ...conversations.filter((c) => !known.has(c.id))]
    })
    return conversations
  }

  deleteConversation = async (conversationId: string): Promise<void> => {
    if (this.#state.conversationId === conversationId) {
      // Nothing of the current turn may be written back under the deleted id.
      this.#detachTurn()
      clearTimeout(this.#persistTimer)
      this.#persistTimer = undefined
      this.newConversation()
    }
    if (this.#state.history === 'server') {
      await this.#api.deleteConversation(conversationId)
    } else if (this.#state.history === 'local') {
      this.#local.deleteConversation(conversationId)
    }
    this.#set({ conversations: this.#state.conversations.filter((c) => c.id !== conversationId) })
  }

  renameConversation = async (conversationId: string, title: string): Promise<void> => {
    // Cut here as the server cuts, so the title shown is the one stored.
    const trimmed = truncateTitle(title.trim())
    if (!trimmed) return
    if (this.#state.history === 'server') {
      await this.#api.renameConversation(conversationId, trimmed)
    } else if (this.#state.history === 'local') {
      this.#local.renameConversation(conversationId, trimmed)
    }
    // Patched in place: the server keeps `updated_at` on a rename, so the list order the
    // next load returns is the one shown now.
    this.#set({
      conversations: this.#state.conversations.map((c) =>
        c.id === conversationId ? { ...c, title: trimmed } : c
      )
    })
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
      const known = new Set(this.#state.messages.map((m) => m.serverId ?? m.id))
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
    this.#leaveConversation()
  }

  // ---- turn internals ----

  async #follow(turn: Turn, onStreamStart: () => void): Promise<unknown> {
    let started = false
    for await (const event of followJob(this.#api, turn.jobId!, { signal: turn.controller.signal })) {
      if (event.type === 'completed') return event.result
      if (!started) {
        started = true
        // Persisted rows for the streaming step would duplicate what is streaming.
        onStreamStart()
      }
      this.#applyEvents(turn, event.events)
    }
    throw new Error('windmill-chat: the job stream ended before the flow completed')
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
      turn.jobIds = await this.#turnJobIds(turn)
      if (!this.#turnActive(turn)) return
      const reconciled = await this.#reconcileTurn(turn)
      if (!this.#turnActive(turn)) return
      if (reconciled) {
        this.#set({ status: 'idle' })
        this.#config.onFinish?.({ conversationId: turn.conversationId, jobId: turn.jobId, messages: this.#state.messages })
        if (isNew) await this.loadConversations().catch(() => {})
        return
      }
      // Server history just proved unreadable: the turn completes as local history.
    }
    let messages = this.#state.messages
    let failed = false
    if (isErrorResult(result)) {
      // The envelope is also a legitimate result shape; the job's own status decides.
      failed = await this.#api
        .getCompletedResult(turn.jobId!, turn.controller.signal)
        .then((r) => r.success === false)
        .catch(() => true)
      if (!this.#turnActive(turn)) return
      if (failed) {
        messages = [...messages, assistantMessage(errorResultMessage(result), false, turn.jobId)]
      }
    }
    if (!failed && !turn.streamedText) {
      const answer = extractChatAnswer(result)
      if (answer !== undefined) {
        messages = [...messages, assistantMessage(answer, true, turn.jobId)]
      }
    }
    this.#set({ messages: finalized(messages), status: 'idle' })
    this.#persistLocal()
    this.#config.onFinish?.({ conversationId: turn.conversationId, jobId: turn.jobId, messages: this.#state.messages })
  }

  /**
   * Folds what the server persisted for the turn into the message list. The rows
   * are written by the worker in their own transactions, each of which can trail
   * the flow's completion, so a streamed message whose row hasn't landed stays and
   * the list is re-read a few times before the rest is kept as streamed.
   * Returns false when the server holds no answer for the turn: history fell back to
   * local, the read was refused or kept failing, or no assistant row has landed. The
   * caller then finishes the turn from the flow result, so an answer is never lost
   * to history. Whether a row counts is read from the message list, not from what
   * this read returned: the turn's polling may have merged the answer already.
   */
  async #reconcileTurn(turn: Turn): Promise<boolean> {
    const answered = () => this.#answered(turn)
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
        if (this.#fallBackToLocal(e)) return false
        const refused = e instanceof WindmillApiError && (e.status === 401 || e.status === 403)
        if (refused || attempt === RECONCILE_ATTEMPTS) return answered()
        await sleep(RECONCILE_DELAY_MS, turn.controller.signal)
        continue
      }
      if (!this.#turnActive(turn)) return true
      this.#mergeRows(rows)
      if (answered() && !this.#state.messages.some((m) => m.pending && m.content)) break
      if (attempt < RECONCILE_ATTEMPTS) await sleep(RECONCILE_DELAY_MS, turn.controller.signal)
    }
    if (!this.#turnActive(turn)) return true
    this.#set({ messages: finalized(this.#state.messages) })
    return answered()
  }

  /**
   * The latest row the turn persisted after its user message is an assistant
   * message. An agent issues each round's text row before that round's tool rows,
   * and a tool row when the tool finishes, so an earlier round's text is followed
   * by a tool row and only the answer closes the turn (the inserts are spawned, so
   * a badly delayed one can invert that order at the cost of the reconcile
   * retries). The content is not compared with the flow result: an image answer, a
   * structured one and a forwarded agent result are all persisted in a shape the
   * result does not reproduce. Rows carrying a job id belong to the turn when the
   * job is one of the turn's, which leaves out an earlier turn whose job outlived
   * `stop()` (a token without `jobs:write` cannot cancel it); a tool row without one
   * (an MCP call runs inside the agent step) belongs to whatever turn is under way.
   */
  #answered(turn: Turn): boolean {
    const messages = this.#state.messages
    const from = messages.findIndex((m) => m.id === turn.userMessageId)
    const ownJob = (m: ChatMessage) =>
      turn.jobIds === undefined || (m.jobId === undefined ? m.role === 'tool' : turn.jobIds.has(m.jobId))
    let latest: ChatMessage | undefined
    for (let i = from + 1; i < messages.length; i++) {
      const m = messages[i]
      if (m.seq === undefined || m.role === 'user' || !ownJob(m)) continue
      if (latest === undefined || m.seq > latest.seq!) latest = m
    }
    return latest?.role === 'assistant'
  }

  /**
   * The flow job plus every step job it ran, the failure and preprocessor steps
   * included (a failure handler's answer is persisted under its own job), and the
   * jobs an agent step's tool calls ran as (a tool row is persisted under its own
   * job too). Unknown when the read fails.
   */
  async #turnJobIds(turn: Turn): Promise<Set<string> | undefined> {
    try {
      const job = await this.#api.getFlowJob(turn.jobId!, turn.controller.signal)
      const ids = new Set([turn.jobId!])
      const status = job.flow_status
      for (const m of [...(status?.modules ?? []), status?.failure_module, status?.preprocessor_module]) {
        if (m?.job) ids.add(m.job)
        for (const j of m?.flow_jobs ?? []) ids.add(j)
        for (const a of m?.agent_actions ?? []) if (a.job_id) ids.add(a.job_id)
      }
      return ids
    } catch (e) {
      if (isAbortError(e)) throw e
      return undefined
    }
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
    this.#config.onError?.(error, { conversationId: turn.conversationId, jobId: turn.jobId })
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
          if (!stopped && this.#turnActive(turn)) this.#mergeRows(rows)
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

  async #syncFromServer(conversationId: string): Promise<void> {
    const rows = await this.#api.listMessages(conversationId, {
      afterSeq: this.#lastSeq(),
      perPage: 100
    })
    if (this.#turn || this.#state.conversationId !== conversationId) return
    this.#mergeRows(rows)
    this.#set({ messages: finalized(this.#state.messages) })
  }

  /**
   * Folds persisted rows into the message list. A row standing for a message the
   * client already shows (same role and text; for a tool, the same tool name, since
   * the server words a failure differently) takes its place under the client's id
   * and keeps what only the stream knew: reasoning, call id, arguments, result.
   * Other rows append in server order. Nothing is dropped: a streamed message
   * outlives a row that never lands.
   */
  #mergeRows(rows: FlowConversationMessage[]): void {
    if (rows.length === 0) return
    const messages = [...this.#state.messages]
    const known = new Set(messages.map((m) => m.serverId ?? m.id))
    for (const row of rows.map(fromRow)) {
      if (known.has(row.id)) continue
      known.add(row.id)
      const i = messages.findIndex(
        (m) =>
          m.seq === undefined &&
          m.role === row.role &&
          (m.content === row.content || (row.tool !== undefined && m.tool?.name === row.tool.name))
      )
      if (i >= 0) {
        const m = messages[i]
        messages[i] = {
          ...row,
          id: m.id,
          reasoning: m.reasoning ?? row.reasoning,
          tool: m.tool ? { ...m.tool, status: row.tool?.status ?? m.tool.status } : row.tool
        }
      } else {
        messages.push(row)
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

  /** Writes the current messages to local history; the conversation entry itself is `#rememberConversation`'s. */
  #persistLocal(): void {
    clearTimeout(this.#persistTimer)
    this.#persistTimer = undefined
    const id = this.#state.conversationId
    if (this.#state.history !== 'local' || !id) return
    this.#local.saveMessages(id, this.#state.messages)
  }

  /**
   * Puts the current conversation at the head of local history. Only a turn moves
   * a conversation there: merely viewing one must not reorder the list.
   */
  #rememberConversation(): void {
    const id = this.#state.conversationId
    if (this.#state.history !== 'local' || !id) return
    const conversation = this.#state.conversations.find((c) => c.id === id)
    if (conversation) this.#local.upsertConversation(conversation)
  }

  /** Whether an unreadable server history should silently become local history. */
  #fallBackToLocal(e: unknown): boolean {
    if (this.#state.history !== 'server' || this.#config.historyExplicit) return false
    if (e instanceof WindmillApiError && (e.status === 401 || e.status === 403)) {
      this.#set({ history: 'local' })
      // The conversation now lives in the browser; list it there like one started local.
      this.#rememberConversation()
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

  /**
   * Leaves the current conversation (for another one, or because the page goes
   * away). A turn still in flight is detached and what it showed so far is kept,
   * written out now rather than on the debounce that may never fire.
   */
  #leaveConversation(): void {
    if (this.#turn) {
      this.#detachTurn()
      this.#set({ messages: finalized(this.#state.messages), status: 'idle' })
    }
    if (this.#persistTimer) this.#persistLocal()
  }

  #set(patch: Partial<ChatState>): void {
    this.#state = { ...this.#state, ...patch }
    for (const listener of this.#listeners) listener(this.#state)
    if (this.#state.history === 'local' && this.#state.conversationId) {
      clearTimeout(this.#persistTimer)
      this.#persistTimer = setTimeout(() => this.#persistLocal(), PERSIST_DEBOUNCE_MS)
    }
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
    serverId: row.id,
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
    updatedAt: row.updated_at,
    isTest: row.is_test
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
