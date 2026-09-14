import { describe, expect, test } from 'bun:test'
import { createChat } from '../src/chat'
import type { ChatOptions } from '../src/types'
import { fetchMock, json, memoryStorage, messageRow, ndjson, sse, text, type Route } from './support'

const BASE = 'http://wm.test'
const FLOW = 'f/chat/agent'

const run: Route = (c) =>
  c.method === 'POST' && c.url.pathname === `/api/w/ws/jobs/run/f/${FLOW}` ? text('job-1') : undefined

const streamPath = '/api/w/ws/jobs_u/getupdate_sse/job-1'

function options(extra: Partial<ChatOptions>, fetch: ChatOptions['fetch']): ChatOptions {
  return { flowPath: FLOW, baseUrl: BASE, workspace: 'ws', fetch, storage: memoryStorage(), ...extra }
}

describe('createChat with local history', () => {
  test('streams text and tool calls, then finalizes and persists the turn', async () => {
    const storage = memoryStorage()
    const { fetch, calls } = fetchMock(run, (c) =>
      c.url.pathname === streamPath
        ? sse([
            { type: 'ping' },
            {
              type: 'update',
              new_result_stream: ndjson(
                { type: 'token_delta', content: 'Let me ' },
                { type: 'tool_call', call_id: 'c1', function_name: 'lookup' },
                { type: 'tool_call_arguments', call_id: 'c1', function_name: 'lookup', arguments: '{"q":1}' }
              ),
              stream_offset: 3,
              flow_stream_job_id: 'agent-job'
            },
            {
              type: 'update',
              new_result_stream: ndjson(
                { type: 'tool_result', call_id: 'c1', function_name: 'lookup', result: '42', success: true },
                { type: 'token_delta', content: 'The answer is 42' }
              ),
              stream_offset: 5,
              completed: true,
              only_result: { output: 'The answer is 42', messages: [] }
            }
          ])
        : undefined
    )
    const chat = createChat(options({ token: 'tok', storage }, fetch))
    const statuses: string[] = []
    chat.subscribe((s) => statuses.push(s.status))

    await chat.sendMessage('what is the answer?', { inputs: { locale: 'fr' } })

    const runCall = calls.find((c) => c.method === 'POST')!
    expect(runCall.url.searchParams.get('memory_id')).toBe(chat.getState().conversationId!)
    expect(runCall.body).toEqual({ locale: 'fr', user_message: 'what is the answer?' })
    expect(runCall.headers.authorization).toBe('Bearer tok')

    const state = chat.getState()
    expect(state.status).toBe('idle')
    expect(statuses).toContain('submitted')
    expect(statuses).toContain('streaming')
    expect(state.messages.map((m) => [m.role, m.content, m.pending])).toEqual([
      ['user', 'what is the answer?', false],
      ['assistant', 'Let me ', false],
      ['tool', 'Used lookup tool', false],
      ['assistant', 'The answer is 42', false]
    ])
    expect(state.messages[2].tool).toEqual({
      callId: 'c1',
      name: 'lookup',
      status: 'success',
      arguments: '{"q":1}',
      result: '42'
    })
    expect(state.conversations).toHaveLength(1)
    expect(state.conversations[0].title).toBe('what is the answer?')

    const reloaded = createChat(options({ token: 'tok', storage }, fetch))
    await reloaded.loadConversations()
    expect(reloaded.getState().conversations.map((c) => c.id)).toEqual([state.conversationId!])
    await reloaded.selectConversation(state.conversationId!)
    expect(reloaded.getState().messages.map((m) => m.content)).toEqual(
      state.messages.map((m) => m.content)
    )
  })

  test('a tool call id reused by a later turn gets its own message', async () => {
    const toolTurn = () =>
      sse([
        {
          type: 'update',
          new_result_stream: ndjson(
            { type: 'tool_call', call_id: 'same-id', function_name: 'lookup' },
            { type: 'tool_result', call_id: 'same-id', function_name: 'lookup', result: '1', success: true },
            { type: 'token_delta', content: 'done' }
          ),
          stream_offset: 3,
          completed: true,
          only_result: { output: 'done', messages: [] }
        }
      ])
    const { fetch } = fetchMock(run, (c) => (c.url.pathname === streamPath ? toolTurn() : undefined))
    const chat = createChat(options({ token: 'tok' }, fetch))
    await chat.sendMessage('one')
    await chat.sendMessage('two')
    expect(chat.getState().messages.map((m) => m.role)).toEqual([
      'user',
      'tool',
      'assistant',
      'user',
      'tool',
      'assistant'
    ])
  })

  test('resumes after a stream timeout from the last offset without re-running the flow', async () => {
    let streamCalls = 0
    const { fetch, calls } = fetchMock(run, (c) => {
      if (c.url.pathname !== streamPath) return undefined
      streamCalls++
      if (streamCalls === 1) {
        return sse([
          { type: 'update', new_result_stream: ndjson({ type: 'token_delta', content: 'Hel' }), stream_offset: 1 },
          { type: 'timeout' }
        ])
      }
      return sse([
        {
          type: 'update',
          new_result_stream: ndjson({ type: 'token_delta', content: 'lo' }),
          stream_offset: 2,
          completed: true,
          only_result: 'Hello'
        }
      ])
    })
    const chat = createChat(options({ token: 'tok' }, fetch))
    await chat.sendMessage('hi')

    expect(calls.filter((c) => c.method === 'POST')).toHaveLength(1)
    const streams = calls.filter((c) => c.url.pathname === streamPath)
    expect(streams).toHaveLength(2)
    expect(streams[0].url.searchParams.get('stream_offset')).toBeNull()
    expect(streams[1].url.searchParams.get('stream_offset')).toBe('1')
    expect(chat.getState().messages.map((m) => m.content)).toEqual(['hi', 'Hello'])
  })

  test('derives the answer from the flow result when nothing streamed', async () => {
    const { fetch } = fetchMock(run, (c) =>
      c.url.pathname === streamPath
        ? sse([{ type: 'update', completed: true, only_result: { windmill_chat_answer: 'From a script' } }])
        : undefined
    )
    const chat = createChat(options({ token: 'tok' }, fetch))
    await chat.sendMessage('hi')
    const [, answer] = chat.getState().messages
    expect(answer.role).toBe('assistant')
    expect(answer.content).toBe('From a script')
    expect(answer.jobId).toBe('job-1')
  })

  test('reports a failed flow as an unsuccessful assistant message', async () => {
    const { fetch } = fetchMock(
      run,
      (c) =>
        c.url.pathname === streamPath
          ? sse([
              {
                type: 'update',
                completed: true,
                only_result: { error: { name: 'ExecutionErr', message: 'boom' } }
              }
            ])
          : undefined,
      (c) =>
        c.url.pathname === '/api/w/ws/jobs_u/completed/get_result_maybe/job-1'
          ? json({ completed: true, success: false, result: { error: { message: 'boom' } } })
          : undefined
    )
    const chat = createChat(options({ token: 'tok' }, fetch))
    await chat.sendMessage('hi')
    const state = chat.getState()
    expect(state.status).toBe('idle')
    expect(state.messages[1]).toMatchObject({ role: 'assistant', success: false, content: 'ExecutionErr: boom' })
  })
})

describe('createChat with server history', () => {
  test('replaces the optimistic turn with the persisted rows', async () => {
    const { fetch, calls } = fetchMock(
      run,
      (c) =>
        c.url.pathname === streamPath
          ? sse([
              {
                type: 'update',
                new_result_stream: ndjson(
                  { type: 'reasoning_token_delta', content: 'hmm' },
                  { type: 'token_delta', content: 'Hello' }
                ),
                stream_offset: 2,
                completed: true,
                only_result: { output: 'Hello', messages: [] }
              }
            ])
          : undefined,
      (c) =>
        c.method === 'GET' && c.url.pathname.endsWith('/messages')
          ? json([
              messageRow(11, 'user', 'hi'),
              messageRow(12, 'assistant', 'Hello', { step_name: 'AI Agent', job_id: 'agent-job' })
            ])
          : undefined,
      (c) =>
        c.url.pathname === '/api/w/ws/flow_conversations/list'
          ? json([
              {
                id: chatId,
                workspace_id: 'ws',
                flow_path: FLOW,
                title: 'hi',
                created_at: '2026-01-01T00:00:00Z',
                updated_at: '2026-01-01T00:00:01Z',
                created_by: 'admin'
              }
            ])
          : undefined
    )
    const chat = createChat(options({}, fetch))
    let chatId = ''
    const unsubscribe = chat.subscribe((s) => {
      chatId = s.conversationId ?? chatId
    })
    await chat.sendMessage('hi')
    unsubscribe()

    const state = chat.getState()
    expect(state.history).toBe('server')
    expect(state.messages.map((m) => [m.id, m.role, m.content, m.pending])).toEqual([
      ['row-11', 'user', 'hi', false],
      ['row-12', 'assistant', 'Hello', false]
    ])
    expect(state.messages[1]).toMatchObject({ reasoning: 'hmm', stepName: 'AI Agent', jobId: 'agent-job', seq: 12 })
    expect(state.conversations.map((c) => c.id)).toEqual([chatId])

    const messagesCall = calls.find((c) => c.url.pathname.endsWith('/messages'))!
    expect(messagesCall.url.pathname).toBe(`/api/w/ws/flow_conversations/${chatId}/messages`)
    expect(messagesCall.headers.authorization).toBeUndefined()
  })

  test('falls back to local history when the credential cannot read conversations', async () => {
    const { fetch } = fetchMock((c) =>
      c.url.pathname === '/api/w/ws/flow_conversations/list' ? text('forbidden', 403) : undefined
    )
    const chat = createChat(options({}, fetch))
    expect(chat.getState().history).toBe('server')
    await chat.loadConversations()
    expect(chat.getState().history).toBe('local')

    const explicit = createChat(options({ history: 'server' }, fetch))
    await expect(explicit.loadConversations()).rejects.toThrow('403')
    expect(explicit.getState().history).toBe('server')
  })
})
