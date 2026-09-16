export type ChatRole = 'user' | 'assistant' | 'tool' | 'system'

/**
 * - `idle`: ready for a message
 * - `submitted`: the message was sent, no answer has started streaming yet
 * - `streaming`: the answer is arriving
 * - `error`: the last turn failed; `error` holds why. Sending again is allowed.
 */
export type ChatStatus = 'idle' | 'submitted' | 'streaming' | 'error'

/**
 * Where conversation history lives.
 * - `server`: Windmill's conversation store. Each Windmill user only sees their own
 *   conversations, so use it with the viewer's own session or a per-user token.
 * - `local`: the browser's storage. Right for a token shared by every visitor.
 * - `none`: nothing is kept beyond the current page.
 */
export type HistoryMode = 'server' | 'local' | 'none'

export interface ToolInvocation {
  callId?: string
  name: string
  /** The arguments the model passed, as a JSON string. */
  arguments?: string
  result?: string
  status: 'running' | 'success' | 'error'
}

export interface ChatMessage {
  id: string
  role: ChatRole
  content: string
  /** The model's reasoning summary, when the provider streams one. */
  reasoning?: string
  /** Set on `tool` messages that came from the live stream. */
  tool?: ToolInvocation
  success: boolean
  createdAt: string
  jobId?: string
  /** The flow step that produced the message. */
  stepName?: string
  /** True while the message is optimistic or still streaming. */
  pending: boolean
  /** Id of the persisted row once the server has it; `id` itself never changes, so list keys stay stable. */
  serverId?: string
  /** The server's cursor for a persisted message; unset for one created on the client. */
  seq?: number
}

export interface Conversation {
  id: string
  title: string | undefined
  createdAt: string
  updatedAt: string
}

export interface ChatState {
  conversationId: string | undefined
  messages: ChatMessage[]
  status: ChatStatus
  error: Error | undefined
  conversations: Conversation[]
  /** Where history is read from. Starts as configured; drops from `server` to `local` when the credential cannot read conversations. */
  history: HistoryMode
  loadingMessages: boolean
  hasMoreMessages: boolean
}

export type TokenSource = string | (() => string | Promise<string>)

export type StorageLike = Pick<Storage, 'getItem' | 'setItem' | 'removeItem'>

export type FetchLike = (input: string | URL | Request, init?: RequestInit) => Promise<Response>

export interface ChatOptions {
  /** Path of a deployed flow with chat mode enabled, e.g. `f/support/assistant`. */
  flowPath: string
  /** Windmill origin, e.g. `https://app.windmill.dev`. Detected inside a raw app. */
  baseUrl?: string
  /** Detected inside a raw app. */
  workspace?: string
  /**
   * A Windmill token, or a function returning one (called before every request, so it
   * can fetch a short-lived token from your backend). Omit it to use the viewer's
   * session: the cookie on the Windmill origin, or a sandboxed raw app's SDK token.
   */
  token?: TokenSource
  /** Defaults to `server` with the viewer's session and `local` with an explicit token. */
  history?: HistoryMode
  /** Extra flow inputs sent with every message, next to `user_message`. */
  inputs?: Record<string, unknown>
  fetch?: FetchLike
  /** Backing store for `local` history. Defaults to `localStorage`. */
  storage?: StorageLike
  /**
   * Namespace for `local` history, e.g. the signed-in user's id. Local history is
   * per browser, per flow; without this, users sharing a browser share it.
   */
  storageKey?: string
  /** Messages fetched per page of server history. */
  pageSize?: number
  /**
   * How often, in milliseconds, the server polls a running turn for the stream
   * (Enterprise; 50 at the fastest, other servers ignore it). Unset, the server
   * relaxes from 100 ms to 3 s over a long turn.
   */
  pollDelayMs?: number
  /**
   * Runs the flow for a turn and returns the job id, instead of the deployed flow at
   * `flowPath`. `args` carries `user_message` and the extra inputs; the run must set
   * `memory_id` to the conversation id for the conversation and its memory to line up.
   * Windmill's own editor uses this to chat with an undeployed flow through a preview run.
   */
  run?: (args: Record<string, unknown>, turn: { conversationId: string; signal: AbortSignal }) => Promise<string>
  /** Called once a turn has its answer (a failed flow included: its error is the answer). */
  onFinish?: (turn: { conversationId: string; jobId?: string; messages: ChatMessage[] }) => void
  /** Called when a turn could not run or be followed; `state.error` holds the same error. */
  onError?: (error: Error, turn: { conversationId: string; jobId?: string }) => void
}

export interface Chat {
  getState(): ChatState
  /** Calls `listener` now and on every change; returns the unsubscribe function (Svelte store contract). */
  subscribe(listener: (state: ChatState) => void): () => void
  /** Sends a message in the current conversation, starting one when there is none. Resolves when the answer is complete. */
  sendMessage(text: string, options?: { inputs?: Record<string, unknown> }): Promise<void>
  /** Stops following the answer and asks Windmill to cancel the run. */
  stop(): Promise<void>
  newConversation(): void
  selectConversation(conversationId: string): Promise<void>
  loadConversations(options?: { page?: number; perPage?: number }): Promise<Conversation[]>
  deleteConversation(conversationId: string): Promise<void>
  loadOlderMessages(): Promise<void>
  /**
   * Points later runs and conversation listings at another flow path, for a flow that was
   * renamed while the chat was open. Conversations already started keep the path they were
   * created under, so listings after the rename show the new path's alone; the open
   * conversation and its turn are untouched.
   */
  setFlowPath(flowPath: string): void
  /** Stops background work (stream, polling) and writes local history out. The chat stays usable. */
  destroy(): void
}
