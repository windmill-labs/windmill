import { describe, expect, it } from 'vitest'
import {
	appendRevealed,
	applyStreamEvent,
	emptyTurnState,
	mergePersistedRows,
	settleTurnRows,
	turnFailed,
	type TurnStep
} from './turnTranscript'
import type { ChatMessage } from './FlowChatManager.svelte'
import type { StreamEvent } from '$lib/components/chat/utils'

function start(): TurnStep {
	return { rows: [], state: emptyTurnState('conv') }
}

let n = 0
const nextId = () => `row-${++n}`

function apply(step: TurnStep, events: StreamEvent[]): TurnStep {
	return events.reduce((acc, event) => applyStreamEvent(acc, event, nextId), step)
}

describe('turn transcript', () => {
	it('keeps thinking that arrives in the same chunk as the tool call it led to', () => {
		// The regression: reasoning revealed and a tool call applied back to back, which is
		// how one SSE chunk delivers "thought about it, then called the tool".
		let step = appendRevealed(start(), 'reasoning', 'Checking the issue first.', nextId)
		step = apply(step, [{ kind: 'tool_call', callId: 'c1', name: 'mcp_linear_get_issue' }])

		expect(step.rows.map((r) => r.message_type)).toEqual(['assistant', 'tool'])
		expect(step.rows[0].reasoning).toBe('Checking the issue first.')
		expect(step.rows[0].streaming).toBe(false)
	})

	it('starts a new answer row after a tool call instead of extending the one before it', () => {
		let step = appendRevealed(start(), 'answer', 'Let me look. ', nextId)
		step = apply(step, [
			{ kind: 'tool_call', callId: 'c1', name: 'get_time' },
			{ kind: 'tool_result', callId: 'c1', name: 'get_time', result: '{}', success: true }
		])
		step = appendRevealed(step, 'answer', 'It is noon.', nextId)

		expect(step.rows.map((r) => r.content)).toEqual([
			'Let me look. ',
			'Used get_time tool',
			'It is noon.'
		])
	})

	it('lands a tool call and its result on one row', () => {
		const step = apply(start(), [
			{ kind: 'tool_call', callId: 'c1', name: 'get_time' },
			{ kind: 'tool_arguments', callId: 'c1', name: 'get_time', arguments: '{"tz":"UTC"}' },
			{ kind: 'tool_execution', callId: 'c1', name: 'get_time' },
			{ kind: 'tool_result', callId: 'c1', name: 'get_time', result: '{"now":1}', success: true }
		])

		const tools = step.rows.filter((r) => r.message_type === 'tool')
		expect(tools).toHaveLength(1)
		expect(tools[0].tool_arguments).toBe('{"tz":"UTC"}')
		expect(tools[0].tool_result).toBe('{"now":1}')
		expect(tools[0].loading).toBe(false)
	})

	it('marks a failed tool call on its row', () => {
		const step = apply(start(), [
			{ kind: 'tool_call', callId: 'c1', name: 'get_time' },
			{ kind: 'tool_result', callId: 'c1', name: 'get_time', result: 'boom', success: false }
		])

		const tool = step.rows.find((r) => r.message_type === 'tool')
		expect(tool?.success).toBe(false)
		// The sentence the server stores for the same call, so a reload does not reword it.
		expect(tool?.content).toBe('Error executing get_time')
	})

	it('grows one answer row as text is revealed', () => {
		let step = appendRevealed(start(), 'answer', 'Hel', nextId)
		step = appendRevealed(step, 'answer', 'lo', nextId)

		expect(step.rows).toHaveLength(1)
		expect(step.rows[0].content).toBe('Hello')
		expect(step.rows[0].streaming).toBe(true)
	})
})

/**
 * Gates the Retry button, which on a flow re-runs the whole thing — side effects included —
 * so it has to mean "this turn produced no answer", not "something inside it went wrong".
 */
describe('turnFailed', () => {
	const row = (over: Partial<ChatMessage>): ChatMessage =>
		({ id: 'x', message_type: 'assistant', content: '', ...over }) as ChatMessage

	it('is false when a tool failed but the agent went on to answer', () => {
		const messages = [
			row({ message_type: 'user' }),
			row({ message_type: 'tool', success: false }),
			row({ message_type: 'assistant', success: true })
		]
		expect(turnFailed(messages, 0)).toBe(false)
	})

	it('is true when the turn ends on a failure', () => {
		const messages = [
			row({ message_type: 'user' }),
			row({ message_type: 'tool', success: true }),
			row({ message_type: 'assistant', success: false })
		]
		expect(turnFailed(messages, 0)).toBe(true)
	})

	it('reports nothing while the turn is still running', () => {
		const messages = [
			row({ message_type: 'user' }),
			row({ message_type: 'tool', success: false }),
			row({ message_type: 'assistant', streaming: true })
		]
		expect(turnFailed(messages, 0)).toBe(false)
	})

	// The window stops at the next user message, so a later turn's failure is not this one's.
	it('does not read past the next user message', () => {
		const messages = [
			row({ message_type: 'user' }),
			row({ message_type: 'assistant', success: true }),
			row({ message_type: 'user' }),
			row({ message_type: 'assistant', success: false })
		]
		expect(turnFailed(messages, 0)).toBe(false)
		expect(turnFailed(messages, 2)).toBe(true)
	})

	it('is false for a turn that has produced nothing yet', () => {
		expect(turnFailed([row({ message_type: 'user' })], 0)).toBe(false)
	})
})

/**
 * The worker writes a turn's rows in transactions it does not wait for, so they land in
 * their own time — after the run reports itself finished, and after the read that follows
 * it. What the reader watched stream in is the only copy of the answer until they do.
 */
describe('folding persisted rows into streamed ones', () => {
	const streamed = (over: Partial<ChatMessage>): ChatMessage =>
		({ id: `temp-${over.content}`, message_type: 'assistant', content: '', ...over }) as ChatMessage
	const stored = (over: Partial<ChatMessage>): ChatMessage =>
		({ id: `db-${over.content}`, message_type: 'assistant', content: '', ...over }) as ChatMessage
	const row = (over: Partial<ChatMessage>): ChatMessage =>
		({ id: 'x', message_type: 'assistant', content: '', ...over }) as ChatMessage

	it('keeps an answer whose row has not landed', () => {
		const rows = mergePersistedRows([streamed({ content: 'the answer' })], [])
		expect(rows.map((r) => r.content)).toEqual(['the answer'])
	})

	it("puts the row in the answer's place rather than beside it", () => {
		const rows = mergePersistedRows(
			[streamed({ content: 'the answer' })],
			[stored({ content: 'the answer', created_seq: 7 })]
		)
		expect(rows).toHaveLength(1)
		// The server's id, which is what moves the poll's cursor past this row.
		expect(rows[0].id).toBe('db-the answer')
		expect(rows[0].created_seq).toBe(7)
	})

	it('keeps a tool call the row does not carry', () => {
		const rows = mergePersistedRows(
			[
				streamed({
					message_type: 'tool',
					content: 'Used get_time tool',
					tool_name: 'get_time',
					tool_arguments: '{"tz":"UTC"}',
					tool_result: '{"now":1}'
				})
			],
			[stored({ message_type: 'tool', content: 'Used get_time tool', job_id: 'job-1' })]
		)
		expect(rows).toHaveLength(1)
		expect(rows[0].job_id).toBe('job-1')
		expect(rows[0].tool_arguments).toBe('{"tz":"UTC"}')
		expect(rows[0].tool_result).toBe('{"now":1}')
		expect(rows[0].tool_name).toBe('get_time')
	})

	it("leaves an older turn's rows to append rather than take an answer's place", () => {
		const rows = mergePersistedRows(
			[streamed({ content: 'the answer' })],
			[stored({ content: 'something asked an hour ago', created_seq: 1 })]
		)
		expect(rows.map((r) => r.content)).toEqual(['the answer', 'something asked an hour ago'])
	})

	/**
	 * A card opens on the call and closes on the result. An agent answering through an output
	 * schema streams the call and never the result, and the worker stores no row for it — so
	 * the card can only be closed by the turn ending.
	 */
	it('pairs a card still on its call with the row stored for that call', () => {
		const rows = mergePersistedRows(
			[
				streamed({
					message_type: 'tool',
					content: 'Running get_time',
					tool_name: 'get_time',
					loading: true
				})
			],
			[stored({ message_type: 'tool', content: 'Used get_time tool' })]
		)
		expect(rows).toHaveLength(1)
		expect(rows[0].content).toBe('Used get_time tool')
	})

	it('leaves a card nothing was stored for, and settles it with the turn', () => {
		const open = [
			streamed({
				message_type: 'tool',
				content: 'Running structured_output',
				tool_name: 'structured_output',
				loading: true
			}),
			streamed({ content: 'the answer', streaming: true })
		]
		const merged = mergePersistedRows(open, [stored({ content: 'the answer' })])
		expect(merged).toHaveLength(2)
		expect(merged[0].loading).toBe(true)

		const settled = settleTurnRows(merged)
		expect(settled[0].loading).toBe(false)
		// The answer was replaced by its stored row, which carries no such flag at all.
		expect(settled[1].streaming).toBeFalsy()
		// And the turn can report itself, which it cannot while a row is still going.
		expect(turnFailed([row({ message_type: 'user' }), ...settled], 0)).toBe(false)
	})

	/**
	 * A stream that restarts on a retried step leaves the row holding both attempts while the
	 * worker stored only the one that answered, and no rule that pairs those reliably also
	 * refuses a row an earlier turn left unread — which would take the live answer's place.
	 * The round shows twice until a reload, which is the lesser of the two.
	 */
	it('appends rather than guessing when the text diverged', () => {
		const rows = mergePersistedRows(
			[streamed({ content: 'half an answerthe whole answer' })],
			[stored({ content: 'the whole answer' })]
		)
		// Both, so the round reads twice. Taking the streamed row's place on anything short of
		// its text would also let a row an earlier turn left unread take the live answer's.
		expect(rows.map((r) => r.content)).toEqual([
			'half an answerthe whole answer',
			'the whole answer'
		])
	})

	it("leaves an older turn's row alone when the text diverged", () => {
		const rows = mergePersistedRows(
			[streamed({ content: 'something from an hour ago' })],
			[stored({ content: 'the whole answer' })]
		)
		expect(rows.map((r) => r.content)).toEqual(['something from an hour ago', 'the whole answer'])
	})

	it('reads a row it has already folded in only once', () => {
		const once = mergePersistedRows(
			[streamed({ content: 'the answer' })],
			[stored({ content: 'the answer' })]
		)
		expect(mergePersistedRows(once, [stored({ content: 'the answer' })])).toHaveLength(1)
	})
})
