import { describe, expect, it } from 'vitest'
import {
	advanceAgentStream,
	emptyAgentStreamProgress,
	formatTokenCount,
	isAgentStream,
	parseAgentResult,
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

describe('agent stream', () => {
	const lines = [
		'{"type":"tool_call","call_id":"c1","function_name":"query_metrics"}',
		'{"type":"tool_result","call_id":"c1","function_name":"query_metrics","result":"{}","success":true}',
		'{"type":"reasoning_token_delta","content":"checking"}',
		'{"type":"token_delta","content":"eu-central-1"}',
		'{"type":"token_delta","content":" is down"}'
	]
	const events = lines.join('\n') + '\n'

	it('recognises an agent stream from its first line only', () => {
		expect(isAgentStream(events)).toBe(true)
		expect(isAgentStream('processing row 1\nprocessing row 2\n')).toBe(false)
		expect(isAgentStream('{"level":"info","msg":"hello"}\n')).toBe(false)
		// No newline yet, so the first line may still be half-written.
		expect(isAgentStream('{"type":"token_delta","content":"a"}')).toBe(false)
	})

	it('folds the token deltas into the answer so far', () => {
		const { stream } = advanceAgentStream(events, emptyAgentStreamProgress())
		expect(stream.answer).toBe('eu-central-1 is down')
		expect(stream.reasoning).toBe('checking')
		expect(stream.tool).toEqual({ name: 'query_metrics', running: false, success: true })
	})

	// The stream only grows, so each poll must fold in the new lines and re-read
	// none of the old ones — the reason this is incremental at all.
	it('resumes where the previous poll stopped', () => {
		const firstPoll = advanceAgentStream(lines.slice(0, 3).join('\n') + '\n', emptyAgentStreamProgress())
		const secondPoll = advanceAgentStream(events, firstPoll)
		expect(secondPoll.consumed).toBe(events.length)
		expect(secondPoll.stream.answer).toBe('eu-central-1 is down')
		expect(secondPoll.stream.reasoning).toBe('checking')
	})

	it('leaves a half-written trailing line for the next poll', () => {
		const partial = advanceAgentStream(`${events}{"type":"token_de`, emptyAgentStreamProgress())
		expect(partial.stream.answer).toBe('eu-central-1 is down')
		const completed = advanceAgentStream(`${events}{"type":"token_delta","content":"!"}\n`, partial)
		expect(completed.stream.answer).toBe('eu-central-1 is down!')
	})

	it('marks a tool still running, and a failed one', () => {
		const started = '{"type":"tool_execution","call_id":"c1","function_name":"fetch"}\n'
		const running = advanceAgentStream(started, emptyAgentStreamProgress())
		expect(running.stream.tool).toEqual({ name: 'fetch', running: true, success: undefined })
		const failed = advanceAgentStream(
			started +
				'{"type":"tool_result","call_id":"c1","function_name":"fetch","result":"boom","success":false}\n',
			running
		)
		expect(failed.stream.tool).toEqual({ name: 'fetch', running: false, success: false })
	})
})

describe('formatTokenCount', () => {
	it('keeps small counts exact and rounds the rest', () => {
		expect(formatTokenCount(940)).toBe('940')
		expect(formatTokenCount(8933)).toBe('8.9k')
		expect(formatTokenCount(48211)).toBe('48k')
	})
})

// The worker streams every iteration's text, and a turn may narrate and call a
// tool at once. Appending across that boundary makes the live answer read
// "I'll checkThe issue is..." and never converge on the finished output.
describe('a stream that narrates before calling a tool', () => {
	it('keeps only the turn that produced the answer', () => {
		const raw = [
			'{"type":"token_delta","content":"Let me check the metrics."}',
			'{"type":"tool_call","call_id":"c1","function_name":"query_metrics"}',
			'{"type":"tool_result","call_id":"c1","function_name":"query_metrics","result":"{}","success":true}',
			'{"type":"token_delta","content":"eu-central-1 is down."}',
			''
		].join('\n')
		expect(advanceAgentStream(raw, emptyAgentStreamProgress()).stream.answer).toBe(
			'eu-central-1 is down.'
		)
	})

	it('drops the narration at the boundary even across polls', () => {
		const first = '{"type":"token_delta","content":"Let me check."}\n'
		const afterCall = first + '{"type":"tool_call","call_id":"c1","function_name":"q"}\n'
		const poll1 = advanceAgentStream(first, emptyAgentStreamProgress())
		expect(poll1.stream.answer).toBe('Let me check.')
		const poll2 = advanceAgentStream(afterCall, poll1)
		expect(poll2.stream.answer).toBe('')
		const poll3 = advanceAgentStream(afterCall + '{"type":"token_delta","content":"Done."}\n', poll2)
		expect(poll3.stream.answer).toBe('Done.')
	})
})
