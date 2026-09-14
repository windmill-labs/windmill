import { describe, expect, test } from 'bun:test'
import { createChat } from '../src/chat'
import type { ChatOptions } from '../src/types'
import { fetchMock, json, memoryStorage, messageRow, ndjson, sse, sseTimed, text, type Route } from './support'

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

  test('a custom run replaces the deployed flow call and still follows the job', async () => {
    const { fetch, calls } = fetchMock((c) =>
      c.url.pathname === streamPath
        ? sse([{ type: 'update', completed: true, only_result: { windmill_chat_answer: 'from preview' } }])
        : undefined
    )
    const seen: unknown[] = []
    const chat = createChat(
      options(
        {
          token: 'tok',
          inputs: { tone: 'kind' },
          run: async (args, turn) => {
            seen.push({ args, conversationId: turn.conversationId, aborted: turn.signal.aborted })
            return 'job-1'
          }
        },
        fetch
      )
    )
    await chat.sendMessage('hi')
    expect(seen).toEqual([{ args: { tone: 'kind', user_message: 'hi' }, conversationId: chat.getState().conversationId, aborted: false }])
    expect(calls.filter((c) => c.method === 'POST')).toHaveLength(0)
    expect(chat.getState().messages.map((m) => m.content)).toEqual(['hi', 'from preview'])
  })

  test('re-attaches from the start when the streaming step is retried under a new sub-job', async () => {
    let streams = 0
    const { fetch, calls } = fetchMock(run, (c) => {
      if (c.url.pathname !== streamPath) return undefined
      streams++
      if (streams === 1) {
        return sse([
          { type: 'update', new_result_stream: ndjson({ type: 'token_delta', content: 'first try ' }), stream_offset: 1, flow_stream_job_id: 'agent-1' },
          // The retried step streams under a new sub-job; the offset above indexes the old one.
          { type: 'update', new_result_stream: ndjson({ type: 'token_delta', content: 'y ' }), stream_offset: 2, flow_stream_job_id: 'agent-2' }
        ])
      }
      return sse([
        {
          type: 'update',
          new_result_stream: ndjson({ type: 'token_delta', content: 'second try' }),
          stream_offset: 1,
          flow_stream_job_id: 'agent-2',
          completed: true,
          only_result: 'second try'
        }
      ])
    })
    const chat = createChat(options({ token: 'tok' }, fetch))
    await chat.sendMessage('hi')
    const streamCalls = calls.filter((c) => c.url.pathname === streamPath)
    expect(streamCalls).toHaveLength(2)
    expect(streamCalls[1].url.searchParams.get('stream_offset')).toBeNull()
    expect(chat.getState().messages.map((m) => m.content)).toEqual(['hi', 'first try second try'])
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

  test('renders a successful result that merely looks like an error envelope', async () => {
    const result = { error: { message: 'domain data' } }
    const { fetch } = fetchMock(
      run,
      (c) => (c.url.pathname === streamPath ? sse([{ type: 'update', completed: true, only_result: result }]) : undefined),
      (c) =>
        c.url.pathname === '/api/w/ws/jobs_u/completed/get_result_maybe/job-1'
          ? json({ completed: true, success: true, result })
          : undefined
    )
    const chat = createChat(options({ token: 'tok' }, fetch))
    await chat.sendMessage('hi')
    expect(chat.getState().messages[1]).toMatchObject({
      role: 'assistant',
      success: true,
      content: JSON.stringify(result, null, 2)
    })
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
    expect(state.messages.map((m) => [m.serverId, m.role, m.content, m.pending])).toEqual([
      ['row-11', 'user', 'hi', false],
      ['row-12', 'assistant', 'Hello', false]
    ])
    // Ids stay the client's, so list keys never remount; the server id rides alongside.
    expect(state.messages.map((m) => m.id.startsWith('pending-'))).toEqual([true, true])
    expect(state.messages[1]).toMatchObject({ reasoning: 'hmm', stepName: 'AI Agent', jobId: 'agent-job', seq: 12 })
    expect(state.conversations.map((c) => c.id)).toEqual([chatId])

    const messagesCall = calls.find((c) => c.url.pathname.endsWith('/messages'))!
    expect(messagesCall.url.pathname).toBe(`/api/w/ws/flow_conversations/${chatId}/messages`)
    expect(messagesCall.headers.authorization).toBeUndefined()
  })

  test('keeps the streamed answer until its row lands, even when a tool row lands first', async () => {
    let messageFetches = 0
    const { fetch } = fetchMock(
      run,
      (c) =>
        c.url.pathname === streamPath
          ? sse([
              {
                type: 'update',
                new_result_stream: ndjson(
                  { type: 'tool_call', call_id: 'c1', function_name: 'lookup' },
                  { type: 'tool_result', call_id: 'c1', function_name: 'lookup', result: '1', success: true },
                  { type: 'token_delta', content: 'Final answer' }
                ),
                stream_offset: 3,
                completed: true,
                only_result: { output: 'Final answer', messages: [] }
              }
            ])
          : undefined,
      (c) => {
        if (!c.url.pathname.endsWith('/messages')) return undefined
        messageFetches++
        // The assistant row is written by a task that trails the tool's.
        return json(
          messageFetches === 1
            ? [messageRow(21, 'user', 'hi'), messageRow(22, 'tool', 'Used lookup tool')]
            : [messageRow(23, 'assistant', 'Final answer')]
        )
      },
      (c) => (c.url.pathname === '/api/w/ws/flow_conversations/list' ? json([]) : undefined)
    )
    const chat = createChat(options({}, fetch))
    await chat.sendMessage('hi')
    expect(messageFetches).toBe(2)
    expect(chat.getState().messages.map((m) => [m.serverId, m.role, m.content, m.pending])).toEqual([
      ['row-21', 'user', 'hi', false],
      ['row-22', 'tool', 'Used lookup tool', false],
      ['row-23', 'assistant', 'Final answer', false]
    ])
    expect(chat.getState().messages[1].tool).toMatchObject({ callId: 'c1', result: '1', status: 'success' })
  })

  test('finishes the turn from the flow result when history falls back mid-turn', async () => {
    const storage = memoryStorage()
    const { fetch } = fetchMock(
      run,
      (c) =>
        c.url.pathname === streamPath
          ? sse([{ type: 'update', completed: true, only_result: { windmill_chat_answer: 'From a script' } }])
          : undefined,
      (c) => (c.url.pathname.includes('/flow_conversations/') ? text('forbidden', 403) : undefined)
    )
    const chat = createChat(options({ storage }, fetch))
    await chat.sendMessage('hi')
    const state = chat.getState()
    expect(state.history).toBe('local')
    expect(state.status).toBe('idle')
    expect(state.messages.map((m) => [m.role, m.content])).toEqual([
      ['user', 'hi'],
      ['assistant', 'From a script']
    ])
    const stored = JSON.parse([...storage.data.values()][0])
    expect(stored.messages[state.conversationId!]).toHaveLength(2)
  })

  test('appends later pages of conversations', async () => {
    const row = (id: string) => ({
      id,
      workspace_id: 'ws',
      flow_path: FLOW,
      title: id,
      created_at: '2026-01-01T00:00:00Z',
      updated_at: '2026-01-01T00:00:00Z',
      created_by: 'admin'
    })
    const { fetch } = fetchMock((c) =>
      c.url.pathname === '/api/w/ws/flow_conversations/list'
        ? json(c.url.searchParams.get('page') === '2' ? [row('c2')] : [row('c1')])
        : undefined
    )
    const chat = createChat(options({}, fetch))
    await chat.loadConversations()
    await chat.loadConversations({ page: 2 })
    expect(chat.getState().conversations.map((c) => c.id)).toEqual(['c1', 'c2'])
  })

  test('a turn started right after stop() is not touched by the stop sync', async () => {
    let jobs = 0
    const { fetch } = fetchMock(
      (c) => (c.method === 'POST' && c.url.pathname.includes('/jobs/run/f/') ? text(`job-${++jobs}`) : undefined),
      (c) =>
        c.url.pathname.endsWith('/getupdate_sse/job-1')
          ? sse([{ type: 'update', new_result_stream: ndjson({ type: 'token_delta', content: 'slow...' }), stream_offset: 1 }])
          : undefined,
      (c) =>
        c.url.pathname.endsWith('/getupdate_sse/job-2')
          ? sse([
              {
                type: 'update',
                new_result_stream: ndjson(
                  { type: 'tool_call', call_id: 'c2', function_name: 'lookup' },
                  { type: 'tool_result', call_id: 'c2', function_name: 'lookup', result: '1', success: true },
                  { type: 'token_delta', content: 'second' }
                ),
                stream_offset: 3,
                completed: true,
                only_result: { output: 'second', messages: [] }
              }
            ])
          : undefined,
      (c) => (c.url.pathname.includes('/queue/cancel/') ? text('ok') : undefined),
      (c) =>
        c.url.pathname.endsWith('/messages')
          ? json([messageRow(31, 'user', 'first'), messageRow(32, 'user', 'second question'), messageRow(33, 'tool', 'Used lookup tool'), messageRow(34, 'assistant', 'second')])
          : undefined,
      (c) => (c.url.pathname === '/api/w/ws/flow_conversations/list' ? json([]) : undefined)
    )
    const chat = createChat(options({}, fetch))
    const first = chat.sendMessage('first')
    // The stream of job-1 never completes: the connection just ends, so the turn keeps waiting.
    await new Promise((r) => setTimeout(r, 50))
    const stopped = chat.stop()
    await first
    const second = chat.sendMessage('second question')
    await stopped
    await second
    const roles = chat.getState().messages.map((m) => `${m.role}${m.pending ? '*' : ''}`)
    expect(roles).toEqual(['user', 'assistant', 'user', 'tool', 'assistant'])
    expect(chat.getState().messages.filter((m) => m.role === 'tool')).toHaveLength(1)
  })

  test('answers from the flow result when only the user row has been persisted', async () => {
    let reads = 0
    const { fetch } = fetchMock(
      run,
      (c) =>
        c.url.pathname === streamPath
          ? sse([{ type: 'update', completed: true, only_result: { windmill_chat_answer: 'From a script' } }])
          : undefined,
      (c) => (c.url.pathname.endsWith('/messages') ? (reads++, json([messageRow(41, 'user', 'hi')])) : undefined),
      (c) => (c.url.pathname === '/api/w/ws/flow_conversations/list' ? json([]) : undefined)
    )
    const chat = createChat(options({}, fetch))
    await chat.sendMessage('hi')
    expect(reads).toBeGreaterThan(1)
    expect(chat.getState().messages.map((m) => [m.role, m.content, m.serverId])).toEqual([
      ['user', 'hi', 'row-41'],
      ['assistant', 'From a script', undefined]
    ])
  })

  test('an answer the poller merged before completion is not appended again', async () => {
    const { fetch } = fetchMock(
      run,
      (c) =>
        c.url.pathname === streamPath
          ? sseTimed([{ type: 'update' }, 1400, { type: 'update', completed: true, only_result: { windmill_chat_answer: 'From a script' } }])
          : undefined,
      (c) =>
        c.url.pathname.endsWith('/messages')
          ? json([messageRow(51, 'user', 'hi'), messageRow(52, 'assistant', 'From a script')])
          : undefined,
      (c) => (c.url.pathname === '/api/w/ws/flow_conversations/list' ? json([]) : undefined)
    )
    const chat = createChat(options({}, fetch))
    await chat.sendMessage('hi')
    expect(chat.getState().messages.map((m) => [m.role, m.content, m.serverId])).toEqual([
      ['user', 'hi', 'row-51'],
      ['assistant', 'From a script', 'row-52']
    ])
  })

  test('an earlier round of a non-streaming agent is not its answer', async () => {
    let reads = 0
    const { fetch } = fetchMock(
      run,
      (c) =>
        c.url.pathname === streamPath
          ? sse([{ type: 'update', completed: true, only_result: { output: 'Final answer', messages: [] } }])
          : undefined,
      (c) => (c.url.pathname.endsWith('/jobs_u/get/job-1') ? json({ flow_status: { modules: [{ job: 'step-1' }] } }) : undefined),
      (c) =>
        c.url.pathname.endsWith('/messages')
          ? json(
              ++reads === 1
                ? [messageRow(91, 'user', 'hi'), messageRow(92, 'assistant', 'Let me check', { job_id: 'step-1' }), messageRow(93, 'tool', 'Used lookup tool', { job_id: 'tool-1' })]
                : [messageRow(94, 'assistant', 'Final answer', { job_id: 'step-1' })]
            )
          : undefined,
      (c) => (c.url.pathname === '/api/w/ws/flow_conversations/list' ? json([]) : undefined)
    )
    const chat = createChat(options({}, fetch))
    await chat.sendMessage('hi')
    expect(reads).toBe(2)
    expect(chat.getState().messages.map((m) => [m.role, m.content, m.serverId])).toEqual([
      ['user', 'hi', 'row-91'],
      ['assistant', 'Let me check', 'row-92'],
      ['tool', 'Used lookup tool', 'row-93'],
      ['assistant', 'Final answer', 'row-94']
    ])
  })

  test('a tool row alone is not the answer of a turn that streamed no text', async () => {
    let reads = 0
    const { fetch } = fetchMock(
      run,
      (c) =>
        c.url.pathname === streamPath
          ? sse([{ type: 'update', completed: true, only_result: { output: 'Answer', messages: [] } }])
          : undefined,
      (c) =>
        c.url.pathname.endsWith('/messages')
          ? json(++reads === 1 ? [messageRow(61, 'user', 'hi'), messageRow(62, 'tool', 'Used lookup tool')] : [messageRow(63, 'assistant', 'Answer')])
          : undefined,
      (c) => (c.url.pathname === '/api/w/ws/flow_conversations/list' ? json([]) : undefined)
    )
    const chat = createChat(options({}, fetch))
    await chat.sendMessage('hi')
    expect(reads).toBe(2)
    expect(chat.getState().messages.map((m) => [m.role, m.content, m.serverId])).toEqual([
      ['user', 'hi', 'row-61'],
      ['tool', 'Used lookup tool', 'row-62'],
      ['assistant', 'Answer', 'row-63']
    ])
  })

  test('a late answer from a stopped job is not taken as the next turn answer', async () => {
    let jobs = 0
    let reads = 0
    const { fetch } = fetchMock(
      (c) => (c.method === 'POST' && c.url.pathname.includes('/jobs/run/f/') ? text(`job-${++jobs}`) : undefined),
      // job-1 never completes: the connection just ends, so the turn keeps waiting.
      (c) => (c.url.pathname.endsWith('/getupdate_sse/job-1') ? sse([{ type: 'update' }]) : undefined),
      (c) =>
        c.url.pathname.endsWith('/getupdate_sse/job-2')
          ? sse([{ type: 'update', completed: true, only_result: { windmill_chat_answer: 'second answer' } }])
          : undefined,
      // The run-only token cannot cancel: job-1 keeps running after stop().
      (c) => (c.url.pathname.includes('/queue/cancel/') ? text('forbidden', 400) : undefined),
      (c) =>
        c.url.pathname.endsWith('/jobs_u/get/job-2')
          ? json({ flow_status: { modules: [{ job: 'step-2' }] } })
          : undefined,
      // Read 1 is stop()'s sync; the stopped job's answer lands after the second user row.
      (c) =>
        c.url.pathname.endsWith('/messages')
          ? json(
              ++reads === 1
                ? [messageRow(71, 'user', 'first')]
                : reads === 2
                  ? [messageRow(72, 'user', 'second'), messageRow(73, 'assistant', 'first answer, late', { job_id: 'step-1' })]
                  : [messageRow(74, 'assistant', 'second answer', { job_id: 'step-2' })]
            )
          : undefined,
      (c) => (c.url.pathname === '/api/w/ws/flow_conversations/list' ? json([]) : undefined)
    )
    const chat = createChat(options({}, fetch))
    const first = chat.sendMessage('first')
    await new Promise((r) => setTimeout(r, 50))
    await chat.stop()
    await first
    await chat.sendMessage('second')
    expect(chat.getState().messages.map((m) => [m.role, m.content, m.serverId])).toEqual([
      ['user', 'first', 'row-71'],
      ['user', 'second', 'row-72'],
      ['assistant', 'first answer, late', 'row-73'],
      ['assistant', 'second answer', 'row-74']
    ])
  })

  test('a failure handler answer is attributed to the turn', async () => {
    let reads = 0
    const { fetch } = fetchMock(
      run,
      (c) =>
        c.url.pathname === streamPath
          ? sse([{ type: 'update', completed: true, only_result: { error: { name: 'ExecutionErr', message: 'boom' } } }])
          : undefined,
      (c) =>
        c.url.pathname.endsWith('/jobs_u/get/job-1')
          ? json({ flow_status: { modules: [{ job: 'step-1' }], failure_module: { job: 'handler-1' } } })
          : undefined,
      (c) =>
        c.url.pathname.endsWith('/messages')
          ? (reads++, json([messageRow(81, 'user', 'hi'), messageRow(82, 'assistant', 'Sorry: boom', { job_id: 'handler-1', success: false })]))
          : undefined,
      (c) => (c.url.pathname === '/api/w/ws/flow_conversations/list' ? json([]) : undefined)
    )
    const chat = createChat(options({}, fetch))
    await chat.sendMessage('hi')
    expect(reads).toBe(1)
    expect(chat.getState().messages.map((m) => [m.role, m.content, m.success, m.serverId])).toEqual([
      ['user', 'hi', true, 'row-81'],
      ['assistant', 'Sorry: boom', false, 'row-82']
    ])
  })

  test('deleting the current local conversation mid-turn leaves nothing behind', async () => {
    const storage = memoryStorage()
    const { fetch } = fetchMock(run, (c) =>
      c.url.pathname === streamPath
        ? sse([{ type: 'update', new_result_stream: ndjson({ type: 'token_delta', content: 'partial' }), stream_offset: 1 }])
        : undefined
    )
    const chat = createChat(options({ token: 'tok', storage }, fetch))
    const turn = chat.sendMessage('hello')
    await new Promise((r) => setTimeout(r, 300))
    const id = chat.getState().conversationId!
    await chat.deleteConversation(id)
    await turn
    await new Promise((r) => setTimeout(r, 400))
    expect(chat.getState().conversations).toEqual([])
    await chat.selectConversation(id)
    expect(chat.getState().messages).toEqual([])
    expect([...storage.data.values()].join('')).not.toContain('hello')
  })

  test('viewing an older local conversation does not reorder history', async () => {
    const storage = memoryStorage()
    const { fetch } = fetchMock(run, (c) =>
      c.url.pathname === streamPath ? sse([{ type: 'update', completed: true, only_result: 'ok' }]) : undefined
    )
    const chat = createChat(options({ token: 'tok', storage }, fetch))
    await chat.sendMessage('older')
    const older = chat.getState().conversationId!
    chat.newConversation()
    await chat.sendMessage('newer')
    const newer = chat.getState().conversationId!
    await chat.selectConversation(older)
    await new Promise((r) => setTimeout(r, 400))
    const again = createChat(options({ token: 'tok', storage }, fetch))
    expect((await again.loadConversations()).map((c) => c.id)).toEqual([newer, older])
  })

  test('destroying the chat mid-turn leaves it idle', async () => {
    const { fetch } = fetchMock(run, (c) =>
      c.url.pathname === streamPath
        ? sse([{ type: 'update', new_result_stream: ndjson({ type: 'token_delta', content: 'partial' }), stream_offset: 1 }])
        : undefined
    )
    const chat = createChat(options({ token: 'tok' }, fetch))
    const turn = chat.sendMessage('hello')
    await new Promise((r) => setTimeout(r, 50))
    expect(chat.getState().status).toBe('streaming')
    chat.destroy()
    await turn
    expect(chat.getState().status).toBe('idle')
    expect(chat.getState().messages.every((m) => !m.pending)).toBe(true)
  })

  test('destroying the chat during a local turn keeps what it showed', async () => {
    const storage = memoryStorage()
    const { fetch } = fetchMock(run, (c) =>
      c.url.pathname === streamPath
        ? sse([{ type: 'update', new_result_stream: ndjson({ type: 'token_delta', content: 'partial' }), stream_offset: 1 }])
        : undefined
    )
    const chat = createChat(options({ token: 'tok', storage }, fetch))
    const turn = chat.sendMessage('hello')
    await new Promise((r) => setTimeout(r, 50))
    const id = chat.getState().conversationId!
    chat.destroy()
    await turn
    const again = createChat(options({ token: 'tok', storage }, fetch))
    await again.loadConversations()
    expect(again.getState().conversations.map((c) => c.id)).toEqual([id])
    await again.selectConversation(id)
    expect(again.getState().messages.map((m) => [m.role, m.content, m.pending])).toEqual([
      ['user', 'hello', false],
      ['assistant', 'partial', false]
    ])
  })

  test('switching conversations keeps what a local turn showed so far', async () => {
    const storage = memoryStorage()
    const { fetch } = fetchMock(run, (c) =>
      c.url.pathname === streamPath
        ? sse([{ type: 'update', new_result_stream: ndjson({ type: 'token_delta', content: 'partial' }), stream_offset: 1 }])
        : undefined
    )
    const chat = createChat(options({ token: 'tok', storage }, fetch))
    const turn = chat.sendMessage('hello')
    await new Promise((r) => setTimeout(r, 50))
    const id = chat.getState().conversationId!
    chat.newConversation()
    await turn
    expect(chat.getState().messages).toEqual([])
    await chat.selectConversation(id)
    expect(chat.getState().messages.map((m) => [m.role, m.content, m.pending])).toEqual([
      ['user', 'hello', false],
      ['assistant', 'partial', false]
    ])
  })

  test('answers from the flow result when server history keeps failing', async () => {
    const { fetch } = fetchMock(
      run,
      (c) =>
        c.url.pathname === streamPath
          ? sse([{ type: 'update', completed: true, only_result: { windmill_chat_answer: 'From a script' } }])
          : undefined,
      (c) => (c.url.pathname.includes('/flow_conversations/') ? text('down', 503) : undefined)
    )
    const chat = createChat(options({ history: 'server' }, fetch))
    await chat.sendMessage('hi')
    const state = chat.getState()
    expect(state.history).toBe('server')
    expect(state.status).toBe('idle')
    expect(state.messages.map((m) => [m.role, m.content])).toEqual([
      ['user', 'hi'],
      ['assistant', 'From a script']
    ])
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
