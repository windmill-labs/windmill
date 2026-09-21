import {
  TurnRunningError,
  WindmillApiError,
  WindmillChatApi,
  type ConversationKind,
  type FlowConversation,
  type FlowConversationMessage,
  type RunningTurn
} from './api'
import { resolveConfig, type ResolvedConfig } from './config'
import { followJob } from './follow'
import { createLocalHistory, type LocalHistory } from './history'
import { uploadAttachments } from './attachments'
import type { AgentStreamEvent } from './stream'
import type {
  Chat,
  ChatMessage,
  ChatOptions,
  ChatState,
  Conversation,
  SendMessageOptions,
  ToolInvocation
} from './types'
import {
  conversationTitle,
  truncateTitle,
  errorResultMessage,
  extractChatAnswer,
  abortError,
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
/** Rows per read of what a turn wrote; a fuller page is read on from its last row. */
const ROWS_PAGE = 100

interface Turn {
  controller: AbortController
  conversationId: string
  /** Id of the turn's user message; the answer is whatever follows it. */
  userMessageId: string
  /** The turn opened the conversation; withdrawing it closes the conversation again. */
  isNew: boolean
  /** The run was asked for. Before that, a failure or a stop withdraws the turn instead of failing it. */
  started: boolean
  /** Both `stop()` and the send's own rejection withdraw; only the first may. */
  withdrawn: boolean
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
  /** Settles once the selected conversation's first page has been read. */
  #selecting: Promise<void> = Promise.resolve()

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

  sendMessage = async (text: string, options: SendMessageOptions = {}): Promise<void> => {
    const content = text.trim()
    if (!content) {
      // A run needs a message; files alone would otherwise be dropped without a word.
      if (options.attachments?.length) throw new Error('windmill-chat: attachments need a message to go with them')
      return
    }
    if (this.#turn) {
      throw new Error('windmill-chat: a message is already being answered; call stop() first')
    }
    const attachments = options.attachments ?? []
    const attachmentsInput = options.attachmentsInput
    if (attachments.length > 0 && !attachmentsInput) {
      throw new Error('windmill-chat: attachments need `attachmentsInput`, the flow input that takes them')
    }
    if (attachmentsInput && !attachmentsInput.multiple && attachments.length > 1) {
      // Uploading all of them would run with the first and leave the rest stranded in storage.
      throw new Error(`windmill-chat: \`${attachmentsInput.name}\` holds one file; got ${attachments.length}`)
    }
    const isNew = this.#state.conversationId === undefined
    const conversationId = this.#state.conversationId ?? randomId()
    const turn: Turn = {
      controller: new AbortController(),
      conversationId,
      userMessageId: `pending-${randomId()}`,
      isNew,
      started: false,
      withdrawn: false,
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
      messages: [
        ...this.#state.messages,
        { id: turn.userMessageId, role: 'user', content, success: true, createdAt: timestamp, pending: true }
      ],
      status: 'submitted',
      error: undefined
    })

    try {
      const args: Record<string, unknown> = { ...this.#config.inputs, ...options.inputs, user_message: content }
      if (attachmentsInput && attachments.length > 0) {
        // Uploaded with the turn already shown as submitted: the message is in the transcript
        // and `stop()` can abort the upload, while a second send is refused as usual.
        const uploaded = await uploadAttachments(this.#api, attachments, randomId(), turn.controller.signal)
        args[attachmentsInput.name] = attachmentsInput.multiple ? uploaded : uploaded[0]
        // Shown on the pending message until its server row replaces it, carrying its own.
        const carried = uploaded.map((u) => ({ input: attachmentsInput.name, s3: u.s3, filename: u.filename }))
        if (this.#turnActive(turn)) {
          this.#set({
            messages: this.#state.messages.map((m) => (m.id === turn.userMessageId ? { ...m, attachments: carried } : m))
          })
        }
      }
      // Nothing may start once stop() or a conversation switch has withdrawn the turn, including
      // a stop from a subscriber told of the attachments just above.
      if (turn.controller.signal.aborted) throw abortError()
      turn.started = true
      // Listed only once the run is asked for: a send that never runs (an upload that failed
      // or was stopped) then has no conversation entry to take back.
      this.#set({
        conversations: [touched, ...this.#state.conversations.filter((c) => c.id !== conversationId)]
      })
      this.#rememberConversation()
      const context = { memoryId: conversationId, conversationId, signal: turn.controller.signal }
      turn.jobId = this.#config.run
        ? await this.#config.run(args, context)
        : await this.#api.runFlow(this.#config.flowPath, args, context)
    } catch (e) {
      try {
        // Nothing of this message reached the server, whether the conversation was already
        // answering one sent elsewhere or an attachment never uploaded: the message is taken
        // back rather than shown as a failed turn, and the caller gets the reason.
        if (e instanceof TurnRunningError || !turn.started) {
          this.#withdrawTurn(turn)
          throw e
        }
        // stop() and a conversation switch abort the turn and settle the state themselves.
        if (turn.controller.signal.aborted || isAbortError(e)) return
        this.#failTurn(turn, e)
        return
      } finally {
        if (this.#turn === turn) this.#turn = undefined
      }
    }
    await this.#followTurn(turn, isNew)
  }

  resumeTurn = async ({ jobId, userSeq }: RunningTurn): Promise<void> => {
    const conversationId = this.#state.conversationId
    if (!conversationId || this.#turn) return
    if (this.#state.history !== 'server') {
      if (this.#state.status === 'submitted') this.#set({ status: 'idle' })
      return
    }
    const turn: Turn = {
      controller: new AbortController(),
      conversationId,
      userMessageId: '',
      jobId,
      // Its run was asked for elsewhere and is already going: there is nothing to withdraw,
      // and the conversation it belongs to is listed.
      isNew: false,
      started: true,
      withdrawn: false,
      streamedText: false
    }
    this.#turn = turn
    this.#set({ status: 'submitted', error: undefined })
    try {
      // The conversation's first page may still be on its way; it would land over the turn.
      await this.#selecting
      if (!this.#turnActive(turn)) return
      // A listing is a snapshot: the turn it named can have ended and another one started
      // since. The newest user message this chat holds names the turn to follow instead —
      // following the one the listing named would drop the newer turn's rows and leave the
      // chat idle while it runs.
      const newest = this.#latestUserAfter(userSeq)
      if (newest) {
        if (!newest.jobId) {
          // Its row is written and its run is not named yet: there is nothing to follow, so
          // it is left to the next listing to name the turn that runs now.
          if (this.#turn === turn) this.#turn = undefined
          this.#set({ status: 'idle' })
          return
        }
        turn.jobId = newest.jobId
        userSeq = newest.seq!
      }
      // The stream replays the turn from its start, so the rows it already wrote go and
      // come back as it replays them. The message that started it stays: it is the turn's
      // anchor, and a long turn can have pushed it off the page this chat opened on.
      let messages = this.#state.messages.filter((m) => m.seq !== undefined && m.seq <= userSeq)
      let user = messages.find((m) => m.seq === userSeq)
      if (!user) {
        const [row] = await this.#api.listMessages(conversationId, {
          afterSeq: userSeq - 1,
          perPage: 1,
          signal: turn.controller.signal
        })
        if (!this.#turnActive(turn)) return
        if (row?.created_seq !== userSeq || row.message_type !== 'user') {
          throw new Error('windmill-chat: the message that started the running turn is gone')
        }
        user = fromRow(row)
        messages = [...messages, user]
      }
      turn.userMessageId = user.id
      this.#set({ messages })
    } catch (e) {
      if (!(turn.controller.signal.aborted || isAbortError(e))) this.#failTurn(turn, e)
      if (this.#turn === turn) this.#turn = undefined
      return
    }
    await this.#followTurn(turn, false)
  }

  async #followTurn(turn: Turn, isNew: boolean): Promise<void> {
    let nextTurn: RunningTurn | undefined
    try {
      const stopPolling = this.#state.history === 'server' ? this.#startPolling(turn) : () => {}
      let result: unknown
      try {
        result = await this.#follow(turn, stopPolling)
      } finally {
        stopPolling()
      }
      nextTurn = await this.#finishTurn(turn, result, isNew)
    } catch (e) {
      if (!turn.started) {
        // Nothing ran: the message is withdrawn rather than shown as a failed turn, and the
        // caller gets the reason (an upload that failed, or the AbortError of a stop()).
        if (this.#turn === turn) this.#turn = undefined
        this.#withdrawTurn(turn)
        throw e
      }
      // stop() and a conversation switch abort the turn and settle the state themselves.
      if (turn.controller.signal.aborted || isAbortError(e)) return
      this.#failTurn(turn, e)
    } finally {
      if (this.#turn === turn) this.#turn = undefined
    }
    if (nextTurn && this.#state.conversationId === turn.conversationId) await this.resumeTurn(nextTurn)
  }

  stop = async (): Promise<void> => {
    const turn = this.#turn
    if (!turn) return
    this.#detachTurn()
    if (!turn.started) {
      // Still uploading its attachments: there is no run to cancel, and the message the
      // reader took back must not stay in the transcript as sent.
      this.#withdrawTurn(turn)
      return
    }
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

  selectConversation = (conversationId: string): Promise<void> => {
    if (conversationId === this.#state.conversationId) return this.#selecting
    const selecting = this.#select(conversationId)
    this.#selecting = selecting.catch(() => {})
    return selecting
  }

  async #select(conversationId: string): Promise<void> {
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
      // Pages count back from the newest row, so rows written since the first page shift
      // newer rows into this one; and a resumed turn drops the rows it replays. Only what
      // is older than everything held belongs above it.
      const oldest = this.#state.messages.reduce<number | undefined>(
        (min, m) => (m.seq !== undefined && (min === undefined || m.seq < min) ? m.seq : min),
        undefined
      )
      const older = rows
        .map(fromRow)
        .filter((m) => !known.has(m.id) && (oldest === undefined || m.seq! < oldest))
      this.#page = page
      this.#set({
        messages: [...older, ...this.#state.messages],
        hasMoreMessages: rows.length === this.#config.pageSize
      })
    } finally {
      if (this.#state.conversationId === conversationId) this.#set({ loadingMessages: false })
    }
  }

  refreshMessages = async (): Promise<void> => {
    const conversationId = this.#state.conversationId
    if (
      !conversationId ||
      this.#turn ||
      this.#state.history !== 'server' ||
      this.#state.loadingMessages
    ) {
      return
    }
    await this.#syncFromServer(conversationId)
  }

  destroy = (): void => {
    this.#leaveConversation()
  }

  // ---- turn internals ----

  async #follow(turn: Turn, onStreamStart: () => void): Promise<unknown> {
    let started = false
    for await (const event of followJob(this.#api, turn.jobId!, { signal: turn.controller.signal })) {
      if (event.type === 'completed') {
        if (event.streamLost) this.#dropCutRound(turn)
        return event.result
      }
      if (!started) {
        started = true
        // Persisted rows for the streaming step would duplicate what is streaming.
        onStreamStart()
      }
      this.#applyEvents(turn, event.events)
    }
    throw new Error('windmill-chat: the job stream ended before the flow completed')
  }

  /**
   * The stream failed while a round's text was arriving, so that text stops wherever the
   * connection did. It goes: the persisted rows or, without them, the flow result give
   * the whole answer instead. Rounds a tool call closed were complete and stay.
   */
  #dropCutRound(turn: Turn): void {
    if (!this.#turnActive(turn)) return
    const cut = turn.assistantId
    turn.assistantId = undefined
    turn.streamedText = false
    if (cut) this.#set({ messages: this.#state.messages.filter((m) => m.id !== cut) })
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
        // Thinking that produced no text led to this call, and is stored on its row.
        const a = turn.assistantId ? messages.findIndex((m) => m.id === turn.assistantId) : -1
        const reasoning = a >= 0 && messages[a].content === '' ? messages.splice(a, 1)[0].reasoning : undefined
        messages.push({
          id: `pending-${randomId()}`,
          role: 'tool',
          content: content ?? '',
          reasoning,
          success: success ?? true,
          createdAt: now(),
          pending: true,
          tool: { callId, name, status: 'running', ...toolPatch }
        })
      }
      turn.assistantId = undefined
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
        // A call completes the round's text: text after it is a new message.
        case 'tool_call':
          upsertTool(event.call_id, event.function_name, { status: 'running' })
          break
        case 'tool_call_arguments':
          upsertTool(event.call_id, event.function_name, { arguments: event.arguments })
          break
        case 'tool_execution':
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

  async #finishTurn(turn: Turn, result: unknown, isNew: boolean): Promise<RunningTurn | undefined> {
    if (!this.#turnActive(turn)) return
    if (this.#state.history === 'server') {
      turn.jobIds = await this.#turnJobIds(turn)
      if (!this.#turnActive(turn)) return
      const reconciled = await this.#reconcileTurn(turn)
      if (!this.#turnActive(turn)) return
      if (reconciled) {
        const nextTurn = this.#nextTurnAfter(turn)
        this.#set({ status: nextTurn ? 'submitted' : 'idle' })
        this.#config.onFinish?.({ conversationId: turn.conversationId, jobId: turn.jobId, messages: this.#state.messages })
        if (isNew) await this.loadConversations().catch(() => {})
        return nextTurn
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
    const nextTurn = this.#state.history === 'server' ? this.#nextTurnAfter(turn) : undefined
    this.#set({ messages: finalized(messages), status: nextTurn ? 'submitted' : 'idle' })
    this.#persistLocal()
    this.#config.onFinish?.({ conversationId: turn.conversationId, jobId: turn.jobId, messages: this.#state.messages })
    return nextTurn
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
        rows = await this.#rowsAfterLastSeq(turn.conversationId, turn.controller.signal)
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
   * result does not reproduce. A row belongs to the turn when it was created after
   * the user message and, when it carries a job id, the job is one of the turn's. The
   * server refuses a turn while the previous run is queued, but an agent writes its
   * answer row from a task the run does not wait for, so that row can still land after
   * the next user message; its job says whose it is. A tool row without a job (an MCP
   * call runs inside the agent step) belongs to the turn under way. Until the user
   * message's own row has been read, its position in the list stands in for its seq.
   */
  #answered(turn: Turn): boolean {
    const messages = this.#state.messages
    const from = messages.findIndex((m) => m.id === turn.userMessageId)
    const userSeq = messages[from]?.seq
    const ownJob = (m: ChatMessage) =>
      turn.jobIds === undefined || (m.jobId === undefined ? m.role === 'tool' : turn.jobIds.has(m.jobId))
    let latest: ChatMessage | undefined
    messages.forEach((m, i) => {
      if (m.seq === undefined || m.role === 'user' || !ownJob(m)) return
      if (userSeq !== undefined ? m.seq <= userSeq : i <= from) return
      if (latest === undefined || m.seq > latest.seq!) latest = m
    })
    return latest?.role === 'assistant'
  }

  /**
   * The flow job plus every step job it ran, the failure and preprocessor steps
   * included (a failure handler's answer is persisted under its own job), and the
   * jobs an agent step's tool calls ran as (a tool row is persisted under its own
   * job too). A failed read is retried like the rows are; unknown when it keeps
   * failing or the credential may not read jobs. Unknown accepts every row after the
   * question: refusing them would leave a token without job access with no turn ever
   * answered, each one finished a second time from its result.
   */
  async #turnJobIds(turn: Turn): Promise<Set<string> | undefined> {
    for (let attempt = 1; ; attempt++) {
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
        const refused = e instanceof WindmillApiError && e.status >= 400 && e.status < 500
        if (refused || attempt === RECONCILE_ATTEMPTS) return undefined
        await sleep(RECONCILE_DELAY_MS, turn.controller.signal)
      }
    }
  }

  /**
   * Take back the user message of a turn that never ran. A conversation it would have opened
   * was never listed (see `sendMessage`), so only the message goes, and, while it is the turn
   * on screen, the busy status. A switch away mid-upload has already written the message to
   * local history, so it is removed there too.
   */
  #withdrawTurn(turn: Turn): void {
    if (turn.withdrawn) return
    turn.withdrawn = true
    const id = turn.conversationId
    const withoutTurn = (messages: ChatMessage[]) => messages.filter((m) => m.id !== turn.userMessageId)
    if (this.#state.conversationId === id) {
      const messages = withoutTurn(this.#state.messages)
      // A turn started since, such as a resend right after Stop, owns the status.
      const newerTurn = this.#turn !== undefined && this.#turn !== turn
      if (newerTurn) {
        this.#set({ messages })
      } else {
        const unopened = turn.isNew && messages.length === 0
        this.#set({ messages, status: 'idle', error: undefined, ...(unopened ? { conversationId: undefined } : {}) })
      }
      this.#persistLocal()
    }
    if (this.#state.history === 'local' && this.#state.conversationId !== id) {
      const stored = withoutTurn(this.#local.getMessages(id))
      if (stored.length > 0) this.#local.saveMessages(id, stored)
      else if (!this.#state.conversations.some((c) => c.id === id)) this.#local.deleteConversation(id)
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
          const rows = await this.#rowsAfterLastSeq(turn.conversationId, signal)
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

  /** Every row created after the newest one held, however many pages that takes. */
  async #rowsAfterLastSeq(conversationId: string, signal?: AbortSignal): Promise<FlowConversationMessage[]> {
    const rows: FlowConversationMessage[] = []
    let afterSeq = this.#lastSeq()
    while (true) {
      const page = await this.#api.listMessages(conversationId, { afterSeq, perPage: ROWS_PAGE, signal })
      rows.push(...page)
      if (page.length < ROWS_PAGE) return rows
      afterSeq = page[page.length - 1].created_seq
    }
  }

  async #syncFromServer(conversationId: string): Promise<void> {
    const rows = await this.#rowsAfterLastSeq(conversationId)
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
      let i = messages.findIndex(
        (m) =>
          m.seq === undefined &&
          m.role === row.role &&
          (m.content === row.content || (row.tool !== undefined && m.tool?.name === row.tool.name))
      )
      // A structured answer streams as the call of the structured-output tool, whose
      // arguments are the answer's text: its row replaces that call. Only past the newest
      // user message, where a stopped turn's identical call cannot be.
      if (i < 0 && row.role === 'assistant') {
        let j = messages.length - 1
        while (j >= 0 && messages[j].role !== 'user') {
          const m = messages[j]
          if (m.seq === undefined && m.role === 'tool' && m.tool?.arguments === row.content) i = j
          j--
        }
      }
      if (i >= 0) {
        const m = messages[i]
        messages[i] = {
          ...row,
          id: m.id,
          reasoning: m.reasoning ?? row.reasoning,
          // The stream's call wins where it has a value; a stream cut short leaves gaps the row fills.
          tool:
            row.role === 'tool' && m.tool
              ? {
                  ...m.tool,
                  arguments: m.tool.arguments ?? row.tool?.arguments,
                  result: m.tool.result ?? row.tool?.result,
                  status: row.tool?.status ?? m.tool.status
                }
              : row.tool
        }
      } else {
        // A tool row nothing streamed, such as a provider-native web search, goes where a
        // reload puts it: after the last message of a lower `seq`, before the streamed answer.
        // Any other row closes the turn, a failure included, and stays last: above a streamed
        // message that never got a row, it would hide the turn's failure.
        let at = messages.length
        for (let j = messages.length - 1; row.role === 'tool' && j >= 0; j--) {
          const seq = messages[j].seq
          if (seq !== undefined && seq < row.seq!) {
            at = j + 1
            break
          }
        }
        messages.splice(at, 0, row)
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

  #latestUserAfter(userSeq: number): ChatMessage | undefined {
    return this.#state.messages.reduce<ChatMessage | undefined>(
      (found, message) =>
        message.role === 'user' && message.seq !== undefined && message.seq > (found?.seq ?? userSeq) ? message : found,
      undefined
    )
  }

  #nextTurnAfter(turn: Turn): RunningTurn | undefined {
    const userSeq = this.#state.messages.find((message) => message.id === turn.userMessageId)?.seq
    if (userSeq === undefined) return undefined
    const next = this.#latestUserAfter(userSeq)
    return next?.jobId && next.seq !== undefined ? { jobId: next.jobId, userSeq: next.seq } : undefined
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
    reasoning: row.reasoning ?? undefined,
    attachments: row.attachments ?? undefined,
    // The call the row carries: the model's arguments and what the model got back. For a
    // failed tool the result is what it failed with, and the row's text names the tool
    // rather than the reason.
    tool: toolName
      ? {
          name: toolName,
          status: success ? 'success' : 'error',
          arguments: row.tool_arguments ?? undefined,
          result: row.tool_result ?? undefined
        }
      : undefined
  }
}

function fromConversation(row: FlowConversation): Conversation {
  return {
    id: row.id,
    title: row.title ?? undefined,
    createdAt: row.created_at,
    updatedAt: row.updated_at,
    isTest: row.is_test,
    runningTurn: row.running_turn
      ? { jobId: row.running_turn.job_id, userSeq: row.running_turn.user_seq }
      : undefined
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
