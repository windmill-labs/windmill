import { describe, expect, test } from 'bun:test'
import { groupTurns, toThreadMessage } from '../src/assistant-ui'
import type { ChatMessage } from '../src/types'

const base = { success: true, createdAt: '2026-01-01T00:00:00Z', pending: false }

describe('assistant-ui conversion', () => {
  test('groups rows into turns and renders tool calls as content parts', () => {
    const messages: ChatMessage[] = [
      { ...base, id: 'u1', role: 'user', content: 'hi' },
      { ...base, id: 't1', role: 'tool', content: 'Used lookup tool', tool: { callId: 'c1', name: 'lookup', status: 'success', arguments: '{"q":1}', result: '42' } },
      { ...base, id: 'a1', role: 'assistant', content: 'The answer is 42', reasoning: 'hmm' },
      { ...base, id: 'u2', role: 'user', content: 'again' },
      { ...base, id: 'a2', role: 'assistant', content: 'partial', pending: true }
    ]
    const turns = groupTurns(messages)
    expect(turns.map((t) => [t.id, t.role, t.messages.length])).toEqual([
      ['u1', 'user', 1],
      ['t1', 'assistant', 2],
      ['u2', 'user', 1],
      ['a2', 'assistant', 1]
    ])
    const answer = toThreadMessage(turns[1])
    expect(answer).toMatchObject({ id: 't1', role: 'assistant', status: { type: 'complete', reason: 'stop' } })
    expect(answer.content).toEqual([
      { type: 'tool-call', toolCallId: 'c1', toolName: 'lookup', args: { q: 1 }, argsText: '{"q":1}', result: 42, isError: false },
      { type: 'reasoning', text: 'hmm' },
      { type: 'text', text: 'The answer is 42' }
    ])
    expect(toThreadMessage(turns[3]).status).toEqual({ type: 'running' })
    expect(toThreadMessage(turns[0])).toMatchObject({ role: 'user', content: [{ type: 'text', text: 'hi' }] })
  })

  test('keeps a stored JSON null result rather than the row text', () => {
    const turn = groupTurns([
      { ...base, id: 't1', role: 'tool', content: 'Used notify tool', tool: { callId: 'c1', name: 'notify', status: 'success', arguments: '{}', result: 'null' } }
    ])[0]
    expect(toThreadMessage(turn).content).toMatchObject([{ type: 'tool-call', toolName: 'notify', result: null }])
  })

  test('marks a failed answer as incomplete', () => {
    const [turn] = groupTurns([{ ...base, id: 'a', role: 'assistant', content: 'boom', success: false }])
    expect(toThreadMessage(turn).status).toEqual({ type: 'incomplete', reason: 'error', error: 'boom' })
  })
})
