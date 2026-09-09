import { describe, expect, it } from 'vitest'
import {
	formatTokenCount,
	parseAgentErrorMessages,
	parseAgentResult,
	parseAgentStream,
	summarizeAgentResult
} from './aiAgentResult'

const envelope = {
	output: 'the answer',
	messages: [
		{ role: 'user', content: 'ask' },
		{ role: 'assistant', content: 'the answer', agent_action: { type: 'message' } }
	],
	usage: { input_tokens: 10, output_tokens: 2, total_tokens: 12 }
}

describe('parseAgentResult', () => {
	it('accepts the envelope with and without its optional keys', () => {
		expect(parseAgentResult(envelope)?.output).toBe('the answer')
		expect(parseAgentResult({ output: 1, messages: [{ role: 'user' }] })?.messages).toHaveLength(1)
	})

	// The signature is the only thing separating an agent result from any other
	// object, so each of these near-misses has to stay a miss.
	it.each([
		['a key outside the envelope', { ...envelope, retries: 2 }],
		['no output', { messages: envelope.messages }],
		['messages that are not a list', { output: 'a', messages: { role: 'user' } }],
		['no messages at all', { output: 'a', messages: [] }],
		['a message without a role', { output: 'a', messages: [{ content: 'ask' }] }],
		['an array', [envelope]],
		['a string', 'output'],
		['null', null]
	])('rejects %s', (_label, value) => {
		expect(parseAgentResult(value)).toBeUndefined()
	})
})

describe('parseAgentErrorMessages', () => {
	it('reads the partial transcript a max-iterations failure carries', () => {
		const messages = parseAgentErrorMessages({
			error: {
				name: 'ExecutionErr',
				message: 'AI agent reached max iterations (10)',
				result: { messages: [{ role: 'user', content: 'ask' }] }
			}
		})
		expect(messages).toHaveLength(1)
	})

	it('ignores an error that carries no transcript', () => {
		expect(
			parseAgentErrorMessages({ error: { name: 'ExecutionErr', message: 'boom' } })
		).toBeUndefined()
	})
})

describe('summarizeAgentResult', () => {
	it('counts the actions and falls back to the parts when no total is reported', () => {
		const summary = summarizeAgentResult({
			output: '',
			messages: [
				{ role: 'user' },
				{ role: 'assistant', agent_action: { type: 'tool_call' } as any },
				{ role: 'assistant', agent_action: { type: 'mcp_tool_call' } as any },
				{ role: 'assistant', agent_action: { type: 'web_search' } },
				{ role: 'assistant', agent_action: { type: 'message' } }
			],
			usage: { input_tokens: 8421, output_tokens: 512, cache_read_input_tokens: 6144 }
		})
		expect(summary).toEqual({
			toolCalls: 2,
			webSearches: 1,
			tokens: 8933,
			cachedTokens: 6144
		})
	})
})

describe('parseAgentStream', () => {
	const events = [
		'{"type":"tool_call","call_id":"c1","function_name":"query_metrics"}',
		'{"type":"tool_result","call_id":"c1","function_name":"query_metrics","result":"{}","success":true}',
		'{"type":"reasoning_token_delta","content":"checking"}',
		'{"type":"token_delta","content":"eu-central-1"}',
		'{"type":"token_delta","content":" is down"}'
	].join('\n')

	it('joins the token deltas into the answer so far', () => {
		const stream = parseAgentStream(events)
		expect(stream?.answer).toBe('eu-central-1 is down')
		expect(stream?.reasoning).toBe('checking')
		expect(stream?.tool).toEqual({ name: 'query_metrics', running: false, success: true })
	})

	it('marks a tool still running', () => {
		expect(
			parseAgentStream('{"type":"tool_execution","call_id":"c1","function_name":"fetch"}')?.tool
		).toEqual({ name: 'fetch', running: true, success: undefined })
	})

	// A poll can cut the last event in half, and any other job may stream something
	// that is not an agent's events at all.
	it('survives a truncated trailing line', () => {
		expect(parseAgentStream(`${events}\n{"type":"token_de`)?.answer).toBe('eu-central-1 is down')
	})

	it('ignores a stream that carries no agent events', () => {
		expect(parseAgentStream('processing row 1\nprocessing row 2')).toBeUndefined()
		expect(parseAgentStream('{"level":"info","msg":"hello"}')).toBeUndefined()
	})
})

describe('formatTokenCount', () => {
	it('keeps small counts exact and rounds the rest', () => {
		expect(formatTokenCount(940)).toBe('940')
		expect(formatTokenCount(8933)).toBe('8.9k')
		expect(formatTokenCount(48211)).toBe('48k')
	})
})
