import type { FetchLike, StorageLike } from '../src/types'

export interface RecordedCall {
  method: string
  url: URL
  headers: Record<string, string>
  body: unknown
}

export type Route = (call: RecordedCall) => Response | Promise<Response> | undefined

/** A fetch whose responses come from the first route that answers; every call is recorded. */
export function fetchMock(...routes: Route[]): { fetch: FetchLike; calls: RecordedCall[] } {
  const calls: RecordedCall[] = []
  const fetch: FetchLike = async (input, init) => {
    const url = new URL(typeof input === 'string' ? input : input instanceof URL ? input.href : input.url)
    const call: RecordedCall = {
      method: init?.method ?? 'GET',
      url,
      headers: Object.fromEntries(
        Object.entries((init?.headers as Record<string, string>) ?? {}).map(([k, v]) => [k.toLowerCase(), v])
      ),
      body: typeof init?.body === 'string' ? JSON.parse(init.body) : undefined
    }
    calls.push(call)
    for (const route of routes) {
      const res = await route(call)
      if (res) return res
    }
    return new Response(`no route for ${call.method} ${url.pathname}`, { status: 404 })
  }
  return { fetch, calls }
}

export function json(value: unknown, status = 200): Response {
  return new Response(JSON.stringify(value), {
    status,
    headers: { 'content-type': 'application/json' }
  })
}

export function text(value: string, status = 200): Response {
  return new Response(value, { status })
}

/** A `text/event-stream` body carrying one `data:` frame per event. */
export function sse(events: object[]): Response {
  return new Response(events.map((e) => `data: ${JSON.stringify(e)}\n\n`).join(''), {
    status: 200,
    headers: { 'content-type': 'text/event-stream' }
  })
}

export function ndjson(...events: object[]): string {
  return events.map((e) => JSON.stringify(e)).join('\n') + '\n'
}

export function memoryStorage(): StorageLike & { data: Map<string, string> } {
  const data = new Map<string, string>()
  return {
    data,
    getItem: (k) => data.get(k) ?? null,
    setItem: (k, v) => void data.set(k, v),
    removeItem: (k) => void data.delete(k)
  }
}

export function messageRow(
  seq: number,
  type: 'user' | 'assistant' | 'tool',
  content: string,
  extra: Record<string, unknown> = {}
) {
  return {
    id: `row-${seq}`,
    conversation_id: 'conv',
    message_type: type,
    content,
    job_id: null,
    created_at: '2026-01-01T00:00:00Z',
    created_seq: seq,
    step_name: null,
    success: true,
    ...extra
  }
}
