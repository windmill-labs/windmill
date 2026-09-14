import type { FetchLike, TokenSource } from './types'

export interface WindmillChatApiOptions {
  baseUrl: string
  workspace: string
  /** Omit to rely on the session cookie of the Windmill origin. */
  token?: TokenSource
  fetch?: FetchLike
}

export class WindmillApiError extends Error {
  constructor(
    message: string,
    readonly status: number
  ) {
    super(message)
    this.name = 'WindmillApiError'
  }
}

export interface FlowConversation {
  id: string
  workspace_id: string
  flow_path: string
  title?: string | null
  created_at: string
  updated_at: string
  created_by: string
}

export interface FlowConversationMessage {
  id: string
  conversation_id: string
  message_type: 'user' | 'assistant' | 'system' | 'tool'
  content: string
  job_id?: string | null
  created_at: string
  created_seq: number
  step_name?: string | null
  success?: boolean
}

export type JobUpdateEvent =
  | {
      type: 'update'
      running?: boolean
      completed?: boolean
      new_result_stream?: string
      stream_offset?: number
      only_result?: unknown
      flow_stream_job_id?: string
    }
  | { type: 'error'; error: string }
  | { type: 'notfound' }
  | { type: 'timeout' }
  | { type: 'ping' }

export interface CompletedJobResult {
  completed: boolean
  success?: boolean
  result?: unknown
}

/** Thin client over the Windmill endpoints a chat-mode flow uses. */
export class WindmillChatApi {
  readonly #baseUrl: string
  readonly #workspace: string
  readonly #token: TokenSource | undefined
  readonly #fetch: FetchLike

  constructor(options: WindmillChatApiOptions) {
    this.#baseUrl = normalizeBaseUrl(options.baseUrl)
    this.#workspace = options.workspace
    this.#token = options.token
    this.#fetch = options.fetch ?? ((input, init) => globalThis.fetch(input, init))
  }

  /** Starts a turn: runs the flow with `memory_id` set to the conversation id. Returns the job id. */
  async runFlow(
    flowPath: string,
    args: Record<string, unknown>,
    options: { memoryId: string; signal?: AbortSignal }
  ): Promise<string> {
    const res = await this.#request(`jobs/run/f/${encodePath(flowPath)}`, {
      method: 'POST',
      query: { memory_id: options.memoryId, skip_preprocessor: 'true' },
      body: args,
      signal: options.signal
    })
    return (await res.text()).trim()
  }

  /**
   * One server-sent-events connection to a job's updates. The server closes it after
   * `TIMEOUT_SSE_STREAM` (a `timeout` event); resume by calling again with the last
   * `stream_offset`, never by re-running the flow.
   */
  async *streamJob(
    jobId: string,
    options: { streamOffset?: number; signal?: AbortSignal } = {}
  ): AsyncGenerator<JobUpdateEvent> {
    const query: Record<string, string> = { fast: 'true', only_result: 'true' }
    if (options.streamOffset !== undefined) {
      query.stream_offset = String(options.streamOffset)
    }
    const res = await this.#request(`jobs_u/getupdate_sse/${encodeURIComponent(jobId)}`, {
      query,
      accept: 'text/event-stream',
      signal: options.signal
    })
    if (!res.body) {
      throw new WindmillApiError('The job update stream has no body', res.status)
    }
    for await (const data of readServerSentEvents(res.body)) {
      try {
        yield JSON.parse(data) as JobUpdateEvent
      } catch {
        // A frame that isn't JSON carries nothing the chat can use.
      }
    }
  }

  async getCompletedResult(jobId: string, signal?: AbortSignal): Promise<CompletedJobResult> {
    const res = await this.#request(
      `jobs_u/completed/get_result_maybe/${encodeURIComponent(jobId)}`,
      { signal }
    )
    return (await res.json()) as CompletedJobResult
  }

  async cancelJob(jobId: string, reason = 'Stopped from the chat'): Promise<void> {
    await this.#request(`jobs_u/queue/cancel/${encodeURIComponent(jobId)}`, {
      method: 'POST',
      body: { reason }
    })
  }

  async listConversations(
    flowPath: string,
    options: { page?: number; perPage?: number; signal?: AbortSignal } = {}
  ): Promise<FlowConversation[]> {
    const res = await this.#request('flow_conversations/list', {
      query: pagination(options, { flow_path: flowPath }),
      signal: options.signal
    })
    return (await res.json()) as FlowConversation[]
  }

  /**
   * Without `afterSeq`: one page counted from the newest message, returned oldest first.
   * With `afterSeq`: the messages created after that cursor, oldest first.
   */
  async listMessages(
    conversationId: string,
    options: { page?: number; perPage?: number; afterSeq?: number; signal?: AbortSignal } = {}
  ): Promise<FlowConversationMessage[]> {
    const extra: Record<string, string> = {}
    if (options.afterSeq !== undefined) extra.after_seq = String(options.afterSeq)
    const res = await this.#request(
      `flow_conversations/${encodeURIComponent(conversationId)}/messages`,
      { query: pagination(options, extra), signal: options.signal }
    )
    return (await res.json()) as FlowConversationMessage[]
  }

  async deleteConversation(conversationId: string): Promise<void> {
    await this.#request(`flow_conversations/delete/${encodeURIComponent(conversationId)}`, {
      method: 'DELETE'
    })
  }

  async #request(
    path: string,
    init: {
      method?: string
      query?: Record<string, string>
      body?: unknown
      accept?: string
      signal?: AbortSignal
    } = {}
  ): Promise<Response> {
    const url = new URL(`${this.#baseUrl}/api/w/${encodeURIComponent(this.#workspace)}/${path}`)
    for (const [k, v] of Object.entries(init.query ?? {})) url.searchParams.set(k, v)

    const headers: Record<string, string> = {}
    if (init.accept) headers['Accept'] = init.accept
    if (init.body !== undefined) headers['Content-Type'] = 'application/json'
    const token = typeof this.#token === 'function' ? await this.#token() : this.#token
    if (token) headers['Authorization'] = `Bearer ${token}`

    const res = await this.#fetch(url.toString(), {
      method: init.method ?? 'GET',
      headers,
      body: init.body === undefined ? undefined : JSON.stringify(init.body),
      // A token must not be paired with ambient cookies; without one, the cookie is
      // the credential and only rides same-origin requests.
      credentials: token ? 'omit' : 'same-origin',
      signal: init.signal
    })
    if (!res.ok) {
      const text = await res.text().catch(() => '')
      throw new WindmillApiError(
        `${init.method ?? 'GET'} ${path} failed (${res.status})${text ? `: ${text}` : ''}`,
        res.status
      )
    }
    return res
  }
}

export function normalizeBaseUrl(baseUrl: string): string {
  return baseUrl.replace(/\/+$/, '').replace(/\/api$/, '')
}

function encodePath(path: string): string {
  return path.split('/').map(encodeURIComponent).join('/')
}

function pagination(
  options: { page?: number; perPage?: number },
  extra: Record<string, string>
): Record<string, string> {
  const query = { ...extra }
  if (options.page !== undefined) query.page = String(options.page)
  if (options.perPage !== undefined) query.per_page = String(options.perPage)
  return query
}

/** Yields the `data` payload of each event in a `text/event-stream` body. */
export async function* readServerSentEvents(
  body: ReadableStream<Uint8Array>
): AsyncGenerator<string> {
  const reader = body.getReader()
  const decoder = new TextDecoder()
  let buffer = ''
  // A CR ending a chunk may be half of a CRLF; it waits for the next chunk.
  let carry = ''
  try {
    while (true) {
      const { value, done } = await reader.read()
      if (done) break
      let text = carry + decoder.decode(value, { stream: true })
      carry = ''
      if (text.endsWith('\r')) {
        carry = '\r'
        text = text.slice(0, -1)
      }
      buffer += text.replace(/\r\n?/g, '\n')
      let end: number
      while ((end = buffer.indexOf('\n\n')) !== -1) {
        const data = eventData(buffer.slice(0, end))
        buffer = buffer.slice(end + 2)
        if (data !== undefined) yield data
      }
    }
    if (carry) buffer += '\n'
    const data = eventData(buffer)
    if (data !== undefined) yield data
  } finally {
    // Closes the connection when the consumer stops early.
    reader.cancel().catch(() => {})
  }
}

function eventData(block: string): string | undefined {
  const lines = block
    .split('\n')
    .filter((line) => line.startsWith('data:'))
    .map((line) => line.slice(line.startsWith('data: ') ? 6 : 5))
  return lines.length > 0 ? lines.join('\n') : undefined
}
