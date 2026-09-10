/**
 * The rows a turn is made of, as its events arrive.
 *
 * A run reports itself as a stream of events — thinking, answer tokens, a tool call, its
 * result — and the transcript is what those become: an assistant row that grows, a card
 * per tool call, the thinking that preceded either. The rules are small but ordered, and
 * getting them wrong is invisible until someone watches a real run: thinking that arrives
 * in the same chunk as the tool call it led to has to keep its row, and a row that a tool
 * interrupted must not take the next answer's text.
 *
 * Kept as plain data in, plain data out — no EventSource, no runes, no toasts — so those
 * rules can be exercised without a socket. FlowChatManager owns the connection and the
 * pacing and calls in here; nothing in this file knows either exists.
 */
import type { ChatMessage } from './FlowChatManager.svelte'
import type { StreamEvent } from '$lib/components/chat/utils'
import { toolSummary } from '$lib/components/chat/utils'

/**
 * What the reducer remembers between events: the assistant row currently open and the
 * text revealed into it, and which row each tool call is writing to.
 */
export type TurnState = {
	conversationId: string
	/** The assistant row taking text right now, or '' when the next text opens a new one. */
	assistantId: string
	content: string
	reasoning: string
	/** Row id per tool call id: a call arrives as up to four events that share one card. */
	toolRowIds: Record<string, string>
}

export type TurnStep = { rows: ChatMessage[]; state: TurnState }

export function emptyTurnState(conversationId: string): TurnState {
	return { conversationId, assistantId: '', content: '', reasoning: '', toolRowIds: {} }
}

function newRow(state: TurnState, id: string, patch: Partial<ChatMessage>): ChatMessage {
	return {
		id,
		content: '',
		created_at: new Date().toISOString(),
		created_seq: 0,
		conversation_id: state.conversationId,
		job_id: '',
		message_type: 'assistant',
		loading: false,
		streaming: false,
		...patch
	} as ChatMessage
}

/** Everything still streaming is finished: nothing further will be appended to it. */
function settleStreamingRows(rows: ChatMessage[]): ChatMessage[] {
	return rows.map((row) => (row.streaming ? { ...row, streaming: false } : row))
}

/**
 * Text revealed into the turn's answer row, opening one if none is. Reasoning opens it
 * too: thinking comes before the first answer token, and a tool call can arrive before
 * that token ever does.
 */
export function appendRevealed(
	{ rows, state }: TurnStep,
	kind: 'answer' | 'reasoning',
	chunk: string,
	newId: () => string
): TurnStep {
	if (chunk === '') return { rows, state }
	const next: TurnState =
		kind === 'answer'
			? { ...state, content: state.content + chunk }
			: { ...state, reasoning: state.reasoning + chunk }
	const reasoning = next.reasoning === '' ? undefined : next.reasoning

	if (next.assistantId === '') {
		const id = newId()
		return {
			rows: [...rows, newRow(next, id, { content: next.content, streaming: true, reasoning })],
			state: { ...next, assistantId: id }
		}
	}
	return {
		rows: rows.map((row) =>
			row.id === next.assistantId ? { ...row, content: next.content, reasoning } : row
		),
		state: next
	}
}

/** Create or patch the row for one tool call; its four events share the one card. */
function upsertToolRow(
	{ rows, state }: TurnStep,
	callId: string,
	patch: Partial<ChatMessage>,
	newId: () => string
): TurnStep {
	const existing = state.toolRowIds[callId]
	if (existing) {
		return {
			rows: rows.map((row) => (row.id === existing ? { ...row, ...patch } : row)),
			state
		}
	}
	const id = newId()
	return {
		rows: [...rows, newRow(state, id, { message_type: 'tool', success: true, ...patch })],
		state: { ...state, toolRowIds: { ...state.toolRowIds, [callId]: id } }
	}
}

/**
 * One event applied. Text events are not handled here: they reach the transcript through
 * `appendRevealed` once the pacing has decided how much of them to show.
 */
export function applyStreamEvent(
	step: TurnStep,
	event: StreamEvent,
	newId: () => string
): TurnStep {
	switch (event.kind) {
		case 'tool_call':
		case 'tool_execution': {
			// The row the answer was going into is finished, and the text that had been
			// revealed into it stays there: the next answer starts its own row. Whatever the
			// pacing still holds must have been flushed before this call, or it would land
			// on the row after the tool card instead of the one before it.
			const settled: TurnStep = {
				rows: settleStreamingRows(step.rows),
				state: { ...step.state, assistantId: '', content: '', reasoning: '' }
			}
			return upsertToolRow(
				settled,
				event.callId,
				{ tool_name: event.name, content: `Running ${event.name}`, loading: true },
				newId
			)
		}
		case 'tool_arguments':
			return upsertToolRow(
				step,
				event.callId,
				{ tool_name: event.name, tool_arguments: event.arguments },
				newId
			)
		case 'tool_result':
			return upsertToolRow(
				step,
				event.callId,
				{
					tool_name: event.name,
					tool_result: event.result,
					content: toolSummary(event.name, event.success),
					success: event.success,
					loading: false
				},
				newId
			)
		default:
			return step
	}
}
