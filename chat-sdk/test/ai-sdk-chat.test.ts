import { describe, expect, test } from 'bun:test'
import { Chat } from '@ai-sdk/react'
import { createWindmillChatTransport } from '../src/ai-sdk'
import { fetchMock, json, ndjson, sse, text } from './support'

const FLOW = 'f/chat/agent'

/** The AI SDK's own chunk processor consuming the transport, as `useChat` would. */
describe('AI SDK Chat over the Windmill transport', () => {
  test('builds the assistant message parts and settles to ready', async () => {
    let jobs = 0
    const { fetch, calls } = fetchMock(
      (c) =>
        c.method === 'POST' && c.url.pathname === `/api/w/ws/jobs/run/f/${FLOW}` ? text(`job-${++jobs}`) : undefined,
      (c) =>
        c.url.pathname.endsWith('/getupdate_sse/job-1')
          ? sse([
              {
                type: 'update',
                new_result_stream: ndjson(
                  { type: 'tool_call', call_id: 'c1', function_name: 'lookup' },
                  { type: 'tool_call_arguments', call_id: 'c1', function_name: 'lookup', arguments: '{"q":1}' },
                  { type: 'tool_result', call_id: 'c1', function_name: 'lookup', result: '42', success: true },
                  { type: 'reasoning_token_delta', content: 'so ' },
                  { type: 'token_delta', content: 'The answer ' },
                  { type: 'token_delta', content: 'is 42' }
                ),
                stream_offset: 6,
                completed: true,
                only_result: { output: 'The answer is 42', messages: [] }
              }
            ])
          : undefined,
      (c) =>
        c.url.pathname.endsWith('/getupdate_sse/job-2')
          ? sse([{ type: 'update', completed: true, only_result: { error: { message: 'boom' } } }])
          : undefined,
      (c) =>
        c.url.pathname.endsWith('/completed/get_result_maybe/job-2') ? json({ completed: true, success: false }) : undefined
    )
    const transport = createWindmillChatTransport({ baseUrl: 'http://wm.test', workspace: 'ws', flowPath: FLOW, token: 'tok', fetch })
    const chat = new Chat({ id: 'e2e-chat', transport })

    await chat.sendMessage({ text: 'what is it?' })

    expect(chat.status).toBe('ready')
    expect(chat.messages.map((m) => m.role)).toEqual(['user', 'assistant'])
    const parts = chat.messages[1].parts
    expect(parts.map((p) => p.type)).toEqual(['dynamic-tool', 'reasoning', 'text'])
    expect(parts[0]).toMatchObject({ toolName: 'lookup', toolCallId: 'c1', state: 'output-available', input: { q: 1 }, output: 42 })
    expect(parts[1]).toMatchObject({ type: 'reasoning', text: 'so ', state: 'done' })
    expect(parts[2]).toMatchObject({ type: 'text', text: 'The answer is 42', state: 'done' })
    // Both turns of the chat ran in the same Windmill conversation.
    const memoryIds = calls.filter((c) => c.method === 'POST').map((c) => c.url.searchParams.get('memory_id'))
    expect(memoryIds[0]).toBe(transport.conversationId('e2e-chat'))

    await chat.sendMessage({ text: 'and now fail' })
    expect(chat.status).toBe('error')
    expect(chat.error?.message).toBe('boom')
    expect(memoryIds.length === 1 || calls.filter((c) => c.method === 'POST')[1].url.searchParams.get('memory_id') === memoryIds[0]).toBe(true)
  })
})
