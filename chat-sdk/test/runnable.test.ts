import { describe, expect, test } from 'bun:test'
import { createChat } from '../src/chat'
import { rawAppRunnable, streamChatTurn, streamLine } from '../src/runnable'
import { parseStreamEvents, type AgentStreamEvent } from '../src/stream'
import { fetchMock, json, memoryStorage, ndjson, sse, text } from './support'

/** What a yielded chunk goes through: the Bun wrapper's log line, then the worker reading it back. */
const relay = (chunk: string) => chunk.replace(/\n/g, '\\n').replace(/\\n/g, '\n')

describe('raw app runnable', () => {
  test('a stream line survives the worker relay', () => {
    const event: AgentStreamEvent = { type: 'token_delta', content: 'one\ntwo \\n "three" \\\\n é' }
    expect(parseStreamEvents(relay(streamLine(event) + streamLine(event)))).toEqual([event, event])
  })

  test('chats through the runnable without a request of its own', async () => {
    const calls: unknown[] = []
    const start = async (args: unknown) => {
      calls.push(args)
      return `job-${calls.length}`
    }
    const streamJob = async (jobId: string, onUpdate?: (u: { new_result_stream?: string }) => void) => {
      const stream = relay(streamLine({ type: 'token_delta', content: `answer ` }) + streamLine({ type: 'token_delta', content: jobId }))
      onUpdate?.({ new_result_stream: stream.slice(0, 10) })
      onUpdate?.({ new_result_stream: stream.slice(10) })
      if (jobId === 'job-3') return { error: { name: 'Error', message: 'agent failed' } }
      return stream
    }
    const chat = createChat({
      ...rawAppRunnable(start, streamJob),
      storage: memoryStorage(),
      fetch: () => Promise.reject(new Error('no request expected'))
    })
    expect(chat.getState().history).toBe('local')
    await chat.sendMessage('hi')
    await chat.sendMessage('again')
    const conversationId = chat.getState().conversationId
    expect(calls).toEqual([
      { user_message: 'hi', conversation_id: conversationId },
      { user_message: 'again', conversation_id: conversationId }
    ])
    expect(chat.getState().messages.map((m) => m.content)).toEqual(['hi', 'answer job-1', 'again', 'answer job-2'])

    await chat.sendMessage('third', {
      attachments: [{ name: 'a.png', data: new Blob(['hi'], { type: 'image/png' }) }],
      attachmentsInput: { name: 'user_attachments', multiple: true }
    })
    expect((calls[2] as { user_attachments: unknown }).user_attachments).toEqual([
      { name: 'a.png', data: 'data:image/png;base64,aGk=' }
    ])
    const messages = chat.getState().messages
    const last = messages[messages.length - 1]
    expect([last.content, last.success]).toEqual(['agent failed', false])
  })

  test('streamChatTurn waits out a running turn and completes a lost stream from the result', async () => {
    expect(await lostStreamTurn('The ans')).toBe('The answer is 42')
    // Lost before a tool call: the narration is not the start of the answer.
    expect(await lostStreamTurn('Let me look that up.')).toBe('Let me look that up.\n\nThe answer is 42')
  }, 30000)
})

async function lostStreamTurn(streamed: string): Promise<string> {
  let runs = 0
  let streams = 0
  const { fetch, calls } = fetchMock(
    (c) => {
      if (c.method !== 'POST' || c.url.pathname !== '/api/w/ws/jobs/run/agent/f/a/agent') return undefined
      return ++runs === 1
        ? json({ error: 'still answering', running_turn: { job_id: 'stopped', user_seq: 3 } }, 409)
        : text('turn')
    },
    (c) =>
      c.url.pathname.endsWith('/getupdate_sse/stopped') ? sse([{ type: 'update', completed: true, only_result: {} }]) : undefined,
    (c) =>
      c.url.pathname.endsWith('/getupdate_sse/turn')
        ? ++streams === 1
          ? sse([{ type: 'update', new_result_stream: ndjson({ type: 'token_delta', content: streamed }), stream_offset: 1 }])
          : text('bad gateway', 502)
        : undefined,
    (c) => (c.url.pathname.endsWith('/job_helpers/upload_s3_file') ? json({ file_key: c.url.searchParams.get('file_key') }) : undefined),
    (c) =>
      c.url.pathname.endsWith('/get_result_maybe/turn')
        ? json({ completed: true, success: true, result: { output: 'The answer is 42', messages: [] } })
        : undefined
  )
  const g = globalThis as { fetch: typeof globalThis.fetch; process: { env: Record<string, string | undefined> } }
  const saved = { fetch: g.fetch, env: g.process.env }
  g.fetch = fetch as typeof globalThis.fetch
  g.process.env = { BASE_URL: 'http://wm.test', WM_WORKSPACE: 'ws', WM_TOKEN: 'tok' }
  try {
    let out = ''
    const user_attachments = [{ name: '../../x.png', data: 'data:image/png;base64,aGk=' }]
    for await (const chunk of streamChatTurn({ agentPath: 'f/a/agent' }, { user_message: 'hi', user_attachments }, 'conv')) out += chunk
    const runs = calls.filter((c) => c.url.pathname.includes('/jobs/run/'))
    expect(runs.map((c) => c.url.searchParams.get('memory_id'))).toEqual(['conv', 'conv'])
    // Stored once, under its own name only, and the run gets the reference.
    const [stored] = (runs[1].body as { user_attachments: { s3: string; filename: string }[] }).user_attachments
    expect(stored.filename).toBe('x.png')
    expect(stored.s3).toMatch(/^windmill_uploads\/chat\/[^/]+\/0\/x\.png$/)
    expect(calls.filter((c) => c.url.pathname.endsWith('/upload_s3_file'))).toHaveLength(1)
    return parseStreamEvents(relay(out)).map((e) => (e.type === 'token_delta' ? e.content : '')).join('')
  } finally {
    g.fetch = saved.fetch
    g.process.env = saved.env
  }
}
