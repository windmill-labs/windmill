import { describe, expect, test } from 'bun:test'
import { TurnRunningError } from '../src/api'
import { createChat } from '../src/chat'
import { rawAppRunnable } from '../src/runnable'
import { memoryStorage, ndjson } from './support'

describe('raw app agent runnable', () => {
  test('chats through the runnable without a request of its own', async () => {
    const calls: unknown[] = []
    let refuse = false
    const start = async (args: unknown) => {
      // As the host relays a 409: the server's body behind the error's stack.
      if (refuse) {
        const body = { error: 'still answering', running_turn: { job_id: 'job-1', user_seq: 1 } }
        throw new Error(`Error: ${JSON.stringify(body)}\n    at x`)
      }
      calls.push(args)
      return `job-${calls.length}`
    }
    const streamJob = async (jobId: string, onUpdate?: (u: { new_result_stream?: string }) => void) => {
      const stream = ndjson({ type: 'token_delta', content: 'answer ' }, { type: 'token_delta', content: jobId })
      onUpdate?.({ new_result_stream: stream.slice(0, 10) })
      onUpdate?.({ new_result_stream: stream.slice(10) })
      if (jobId === 'job-3') return { error: { name: 'Error', message: 'agent failed' } }
      return { output: `answer ${jobId}`, messages: [] }
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
      { user_message: 'hi', memory_id: conversationId },
      { user_message: 'again', memory_id: conversationId }
    ])
    expect(chat.getState().messages.map((m) => m.content)).toEqual(['hi', 'answer job-1', 'again', 'answer job-2'])

    await chat.sendMessage('third')
    const messages = chat.getState().messages
    const last = messages[messages.length - 1]
    expect([last.content, last.success]).toEqual(['agent failed', false])

    // A turn still running elsewhere takes the message back rather than failing a turn.
    refuse = true
    await expect(chat.sendMessage('fourth')).rejects.toBeInstanceOf(TurnRunningError)
    expect(chat.getState().messages).toEqual(messages)
  })
})
