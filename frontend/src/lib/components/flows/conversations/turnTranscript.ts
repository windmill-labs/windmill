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

/** A row the stream wrote, still standing in for one the server has not handed back yet. */
function isStreamedRow(row: ChatMessage): boolean {
	// User rows are never read back — the poll drops them — so theirs is the only temp id
	// that is permanent, and nothing the server sends can stand for one.
	return row.id.startsWith('temp-') && row.message_type !== 'user'
}

/**
 * Whether a persisted tool row is the one a streamed tool card was opened for.
 *
 * A card opens on the call and is worded `Running X` until its result arrives; a card whose
 * result never arrives keeps that wording and would otherwise never meet the row the worker
 * stored for the same call. The worker words every tool row from the tool, so the two
 * sentences it can have are known exactly and are compared as text rather than parsed.
 */
function namesTool(content: string, toolName: string | undefined): boolean {
	if (!toolName) return false
	return content === `Used ${toolName} tool` || content === `Error executing ${toolName}`
}

/**
 * Which streamed row a persisted one stands for, or -1 when it stands for none.
 *
 * What the row says, and for a tool the tool it names. Nothing looser: a read walks the
 * conversation forward from wherever the transcript left off, so it carries rows written
 * before this turn as readily as its own — a row an earlier turn left unread among them —
 * and any rule that pairs by position or by kind alone would let one of those take the place
 * of the answer being streamed, which is the one thing worse than showing it twice.
 *
 * Showing it twice is what happens when the two texts disagree: a stream restarting on a
 * retried step replays the round from its beginning, so the row holds both attempts while
 * the worker stored only the one that answered. Neither text contains the other in any
 * reliable way, so the stored row is appended and the reader sees the round twice until the
 * page is reloaded.
 */
function indexOfStreamedRowFor(rows: ChatMessage[], row: ChatMessage): number {
	const eligible = (held: ChatMessage) =>
		isStreamedRow(held) && held.message_type === row.message_type
	const sameText = rows.findIndex((held) => eligible(held) && held.content === row.content)
	if (sameText >= 0) return sameText
	return rows.findIndex((held) => eligible(held) && namesTool(row.content, held.tool_name))
}

/**
 * Fold the rows the server has stored into the rows on screen.
 *
 * A turn writes its rows twice: as the stream reveals them, and again by the worker in
 * transactions of its own — spawned, so they can trail the flow's own completion. A
 * persisted row that stands for one already on screen takes its place and keeps what only
 * the stream knew: a tool's call and result, the model's thinking. One that stands for
 * nothing on screen is appended in server order.
 *
 * Nothing is dropped. A streamed row outlives a persisted row that never lands, which is
 * what keeps an answer the reader watched arrive from disappearing when the write behind it
 * is late — the chat does not wait for that write and so can never conclude it is not coming.
 */
export function mergePersistedRows(held: ChatMessage[], arriving: ChatMessage[]): ChatMessage[] {
	const rows = [...held]
	const known = new Set(rows.map((row) => row.id))
	for (const row of arriving) {
		if (known.has(row.id)) continue
		known.add(row.id)
		const index = indexOfStreamedRowFor(rows, row)
		if (index < 0) {
			rows.push(row)
			continue
		}
		const streamed = rows[index]
		// The persisted row wins on everything it has an answer for — its id above all, which
		// is what moves the poll's cursor past it. The stream's details stay where it has none:
		// a tool with a job of its own stores no call on the row, and thinking reaches the row
		// only when the provider streamed it.
		rows[index] = {
			...row,
			tool_name: row.tool_name ?? streamed.tool_name,
			tool_arguments: row.tool_arguments ?? streamed.tool_arguments,
			tool_result: row.tool_result ?? streamed.tool_result,
			reasoning: row.reasoning ?? streamed.reasoning
		}
	}
	return rows
}

/**
 * Nothing is being written any more: whatever a turn left mid-flight is as finished as it
 * is going to get.
 *
 * A card can be opened by an event whose closing event never comes — an agent answering
 * through an output schema streams the call and no result, and a tool the model named but
 * the flow does not have fails before one. The worker stores no row for either, so nothing
 * arriving later can stand for them, and a card left spinning would spin for as long as the
 * conversation is open. It also gates `turnFailed`, which reports nothing while a row is
 * still going — so a turn that ends this way would never offer Retry.
 */
export function settleTurnRows(rows: ChatMessage[]): ChatMessage[] {
	return rows.map((row) =>
		row.streaming || row.loading ? { ...row, streaming: false, loading: false } : row
	)
}

/**
 * Whether the turn a user message started ended without an answer.
 *
 * Read from the turn's last row and no other. A tool that fails mid-turn is handed back to
 * the agent, which routinely recovers and answers, so an unsuccessful tool row says nothing
 * about the turn — and this drives the Retry button, which in the copilot means "the request
 * never went through" rather than "something inside it went wrong". Offering it for a turn
 * that answered would invite running the whole flow a second time, side effects and all.
 *
 * A turn with no row at all is not a failure to report: a run that dies anywhere still ends
 * in an assistant row carrying the error (`worker_flow.rs`, on flow completion), so the only
 * turns with nothing after the user message are the ones still waiting for their first row.
 */
export function turnFailed(messages: ChatMessage[], userIndex: number): boolean {
	let last: ChatMessage | undefined
	for (let i = userIndex + 1; i < messages.length; i++) {
		const message = messages[i]
		if (message.message_type === 'user') break
		// Still going, so the turn has no outcome to report yet.
		if (message.streaming || message.loading) return false
		last = message
	}
	return last?.success === false
}
