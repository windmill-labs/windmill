import { describe, expect, test } from 'bun:test'
import { createChat } from '../src/chat'
import { rawAppRunnable, streamLine } from '../src/runnable'
import { parseStreamEvents, type AgentStreamEvent } from '../src/stream'
import { memoryStorage } from './support'

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

    await chat.sendMessage('third')
    const messages = chat.getState().messages
    const last = messages[messages.length - 1]
    expect([last.content, last.success]).toEqual(['agent failed', false])
  })
})
