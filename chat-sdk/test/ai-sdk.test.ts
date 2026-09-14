import { describe, expect, test } from 'bun:test'
import type { UIMessage, UIMessageChunk } from 'ai'
import { createWindmillChatTransport, toUIMessages } from '../src/ai-sdk'
import type { ChatMessage } from '../src/types'
import { fetchMock, json, ndjson, sse, text, type Route } from './support'

const FLOW = 'f/chat/agent'
const run: Route = (c) =>
  c.method === 'POST' && c.url.pathname === `/api/w/ws/jobs/run/f/${FLOW}` ? text('job-1') : undefined
const streamPath = '/api/w/ws/jobs_u/getupdate_sse/job-1'

const userMessage = (text: string): UIMessage => ({ id: 'u1', role: 'user', parts: [{ type: 'text', text }] })

async function collect(stream: ReadableStream<UIMessageChunk>): Promise<UIMessageChunk[]> {
  const chunks: UIMessageChunk[] = []
  const reader = stream.getReader()
  while (true) {
    const { value, done } = await reader.read()
    if (done) return chunks
    chunks.push(value)
  }
}

describe('createWindmillChatTransport', () => {
  test('maps an agent turn to AI SDK chunks and derives the conversation from the chat id', async () => {
    const { fetch, calls } = fetchMock(run, (c) =>
      c.url.pathname === streamPath
        ? sse([
            {
              type: 'update',
              new_result_stream: ndjson(
                { type: 'reasoning_token_delta', content: 'think' },
                { type: 'token_delta', content: 'Let me ' },
                { type: 'tool_call', call_id: 'c1', function_name: 'lookup' },
                { type: 'tool_call_arguments', call_id: 'c1', function_name: 'lookup', arguments: '{"q":1}' },
                { type: 'tool_execution', call_id: 'c1', function_name: 'lookup' }
              ),
              stream_offset: 5
            },
            {
              type: 'update',
              new_result_stream: ndjson(
                { type: 'tool_result', call_id: 'c1', function_name: 'lookup', result: '{"answer":42}', success: true },
                { type: 'token_delta', content: '42' }
              ),
              stream_offset: 7,
              completed: true,
              only_result: { output: '42', messages: [] }
            }
          ])
        : undefined
    )
    const transport = createWindmillChatTransport({
      baseUrl: 'http://wm.test',
      workspace: 'ws',
      flowPath: FLOW,
      token: 'tok',
      inputs: { tone: 'kind' },
      fetch
    })
    const chunks = await collect(
      await transport.sendMessages({
        trigger: 'submit-message',
        chatId: 'chat-abc',
        messageId: undefined,
        messages: [userMessage('what is it?')],
        abortSignal: undefined,
        body: { locale: 'fr' }
      })
    )

    const runCall = calls.find((c) => c.method === 'POST')!
    expect(runCall.body).toEqual({ tone: 'kind', locale: 'fr', user_message: 'what is it?' })
    expect(runCall.url.searchParams.get('memory_id')).toBe(await transport.conversationId('chat-abc'))
    expect(await transport.conversationId('chat-abc')).toMatch(/^[0-9a-f-]{36}$/)
    expect(await transport.conversationId('4D6E5C8C-2C3B-4E1A-9F31-0B2E6F1C9D10')).toBe(
      '4d6e5c8c-2c3b-4e1a-9f31-0b2e6f1c9d10'
    )

    const shape = chunks.map((c) => ('delta' in c ? `${c.type}:${c.delta}` : c.type))
    expect(shape).toEqual([
      'start',
      'reasoning-start',
      'reasoning-delta:think',
      'text-start',
      'text-delta:Let me ',
      'reasoning-end',
      'text-end',
      'tool-input-start',
      'tool-input-available',
      'tool-output-available',
      'text-start',
      'text-delta:42',
      'text-end',
      'finish'
    ])
    expect(chunks.find((c) => c.type === 'tool-input-available')).toMatchObject({
      toolCallId: 'c1',
      toolName: 'lookup',
      input: { q: 1 },
      dynamic: true
    })
    expect(chunks.find((c) => c.type === 'tool-output-available')).toMatchObject({ output: { answer: 42 } })
    // The job finished, so there is nothing to reconnect to.
    expect(await transport.reconnectToStream({ chatId: 'chat-abc' })).toBeNull()
  })

  test('answers from the flow result when nothing streamed, and reports a failed flow as an error', async () => {
    let turn = 0
    const { fetch } = fetchMock(
      run,
      (c) => {
        if (c.url.pathname !== streamPath) return undefined
        turn++
        return sse([
          turn === 1
            ? { type: 'update', completed: true, only_result: { windmill_chat_answer: 'From a script' } }
            : { type: 'update', completed: true, only_result: { error: { name: 'ExecutionErr', message: 'boom' } } }
        ])
      },
      (c) =>
        c.url.pathname === '/api/w/ws/jobs_u/completed/get_result_maybe/job-1'
          ? json({ completed: true, success: false })
          : undefined
    )
    const transport = createWindmillChatTransport({ baseUrl: 'http://wm.test', workspace: 'ws', flowPath: FLOW, fetch })
    const send = () =>
      transport.sendMessages({
        trigger: 'submit-message',
        chatId: 'c',
        messageId: undefined,
        messages: [userMessage('hi')],
        abortSignal: undefined
      })
    const first = await collect(await send())
    expect(first.map((c) => ('delta' in c ? c.delta : c.type))).toEqual(['start', 'text-start', 'From a script', 'text-end', 'finish'])
    const second = await collect(await send())
    expect(second.map((c) => c.type)).toEqual(['start', 'error'])
    expect(second[1]).toMatchObject({ errorText: 'ExecutionErr: boom' })
  })

  test('refuses attachments with a clear error', async () => {
    const transport = createWindmillChatTransport({ baseUrl: 'http://wm.test', workspace: 'ws', flowPath: FLOW, fetch: fetchMock().fetch })
    await expect(
      transport.sendMessages({
        trigger: 'submit-message',
        chatId: 'c',
        messageId: undefined,
        messages: [{ id: 'u', role: 'user', parts: [{ type: 'file', mediaType: 'image/png', url: 'data:...' }] }],
        abortSignal: undefined
      })
    ).rejects.toThrow('attachments are not supported')
  })
})

describe('toUIMessages', () => {
  test('folds a turn into one assistant message with reasoning, tool and text parts', () => {
    const base = { success: true, createdAt: '2026-01-01T00:00:00Z', pending: false }
    const messages: ChatMessage[] = [
      { ...base, id: 'u1', role: 'user', content: 'hi' },
      { ...base, id: 't1', role: 'tool', content: 'Used lookup tool', tool: { callId: 'c1', name: 'lookup', status: 'success', arguments: '{"q":1}', result: '42' } },
      { ...base, id: 'a1', role: 'assistant', content: 'The answer is 42', reasoning: 'hmm' },
      { ...base, id: 'u2', role: 'user', content: 'thanks' },
      { ...base, id: 't2', role: 'tool', content: 'Error executing lookup', success: false, tool: { name: 'lookup', status: 'error' } }
    ]
    const ui = toUIMessages(messages)
    expect(ui.map((m) => [m.id, m.role, m.parts.map((p) => p.type)])).toEqual([
      ['u1', 'user', ['text']],
      ['t1', 'assistant', ['dynamic-tool', 'reasoning', 'text']],
      ['u2', 'user', ['text']],
      ['t2', 'assistant', ['dynamic-tool']]
    ])
    expect(ui[1].parts[0]).toMatchObject({ toolCallId: 'c1', toolName: 'lookup', state: 'output-available', input: { q: 1 }, output: 42 })
    expect(ui[3].parts[0]).toMatchObject({ state: 'output-error', errorText: 'Error executing lookup' })
  })
})
