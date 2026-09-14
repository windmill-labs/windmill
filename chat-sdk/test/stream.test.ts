import { describe, expect, test } from 'bun:test'
import { readServerSentEvents } from '../src/api'
import { createStreamEventParser, parseStreamEvents } from '../src/stream'
import { ndjson } from './support'

describe('parseStreamEvents', () => {
  test('keeps agent events and skips other lines', () => {
    const events = parseStreamEvents(
      ndjson(
        { type: 'token_delta', content: 'Hi' },
        { type: 'reasoning_token_delta', content: 'thinking' },
        { type: 'something_else', content: 'x' },
        { type: 'tool_result', call_id: 'c1', function_name: 'lookup', result: '42', success: true }
      ) + 'not json\n'
    )
    expect(events.map((e) => e.type)).toEqual(['token_delta', 'reasoning_token_delta', 'tool_result'])
  })
})

describe('createStreamEventParser', () => {
  test('holds an incomplete line until the rest arrives', () => {
    const parser = createStreamEventParser()
    const line = JSON.stringify({ type: 'token_delta', content: 'Hello' })
    expect(parser.push(line.slice(0, 10))).toEqual([])
    expect(parser.push(line.slice(10) + '\n' + '{"type":"token_delta",')).toEqual([
      { type: 'token_delta', content: 'Hello' }
    ])
    expect(parser.push('"content":"!"}')).toEqual([])
    expect(parser.flush()).toEqual([{ type: 'token_delta', content: '!' }])
  })
})

describe('readServerSentEvents', () => {
  test('splits frames that straddle chunks and normalizes CRLF', async () => {
    const chunks = ['data: {"a":1}\r\n\r\ndata: {"b"', ':2}\n\ndata: first\ndata: second\n\n', 'data: {"c":3}']
    const encoder = new TextEncoder()
    const body = new ReadableStream<Uint8Array>({
      start(controller) {
        for (const c of chunks) controller.enqueue(encoder.encode(c))
        controller.close()
      }
    })
    const frames: string[] = []
    for await (const data of readServerSentEvents(body)) frames.push(data)
    expect(frames).toEqual(['{"a":1}', '{"b":2}', 'first\nsecond', '{"c":3}'])
  })
})
