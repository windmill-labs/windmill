import type { DisplayMessage } from '$lib/components/copilot/chat/shared'

/**
 * The attachment payloads a mirror frame does not carry, and how to put them
 * back. A frame goes out several times a second for the whole turn, and the
 * message holding an image or a pasted file sits in its tail unchanged
 * throughout — so shipping the payloads means re-cloning and re-broadcasting
 * the same megabytes on every tick (the caps allow roughly 16MB of base64 image
 * plus 5MB of file content on a single message, before pastes).
 *
 * Every heavy field is listed here rather than at its own call site: they are
 * spread across the message union, and one that goes unlisted silently
 * reintroduces the whole regression.
 */

/** Emptied in place, keeping the sibling fields the chip renders from. */
const BLANKED_ITEM_FIELDS = [
	['files', 'content'],
	['pastes', 'content']
] as const

/** Dropped whole: the bubble renders one <img> per entry with no per-image
 *  guard, so an emptied url would show a broken image where the real one is
 *  about to appear. */
const DROPPED_LIST_FIELDS = ['images'] as const

/** Dropped whole, and unbounded in a way the others are not: a tool's result or
 *  logs can be a whole query result or job output, and a frame re-clones and
 *  re-broadcasts the running turn's tail several times a second. A watching tab
 *  shows the tool card without its output until the turn-end re-read supplies
 *  it, which is the same trade the images above make. */
const DROPPED_FIELDS = ['result', 'logs'] as const

/** Emptied outright; guarded at the render site, so it renders nothing. */
const BLANKED_FIELDS = ['imageUrl'] as const

function isHeavy(message: DisplayMessage): boolean {
	const m = message as Record<string, any>
	return (
		DROPPED_LIST_FIELDS.some((f) => m[f]?.length) ||
		DROPPED_FIELDS.some((f) => m[f] !== undefined) ||
		BLANKED_ITEM_FIELDS.some(([f]) => m[f]?.length) ||
		BLANKED_FIELDS.some((f) => m[f])
	)
}

/** Strip the bytes out of a transcript bound for a mirror frame. */
export function withoutHeavyPayloads(messages: DisplayMessage[]): DisplayMessage[] {
	return messages.map((message) => {
		if (!isHeavy(message)) return message
		const stripped: Record<string, any> = { ...message }
		for (const f of DROPPED_LIST_FIELDS) delete stripped[f]
		for (const f of DROPPED_FIELDS) delete stripped[f]
		for (const [f, item] of BLANKED_ITEM_FIELDS) {
			if (stripped[f]?.length) stripped[f] = stripped[f].map((v: any) => ({ ...v, [item]: '' }))
		}
		for (const f of BLANKED_FIELDS) if (stripped[f]) stripped[f] = ''
		return stripped as DisplayMessage
	})
}

/** How many of the newest messages a frame carries when it is not sending the
 *  whole transcript. In-place edits to already-rendered cards (a tool card
 *  settling) land within a few messages of the end, so a short tail carries them
 *  while keeping the frame bounded on a long conversation. */
const MIRROR_TAIL = 10

/**
 * Where the tail a frame carries starts.
 *
 * `full` sends the whole transcript and overrides everything else — it is what
 * answers a resync, and a resync that came back partial would fail the receiver's
 * prefix check again and ask for another one, forever.
 *
 * Otherwise the frame reaches no further back than `turnStart`, the index of the
 * running turn's first message: everything from there on is either new this turn
 * or a stripped copy the receiver already got from an earlier frame of the same
 * turn, so overwriting it can never destroy a message held complete from the
 * store.
 */
export function mirrorFrameStart({
	total,
	turnStart,
	full
}: {
	total: number
	turnStart: number
	full: boolean
}): number {
	if (full) return 0
	return Math.max(turnStart, Math.max(0, total - MIRROR_TAIL))
}

/**
 * Whether a receiver can splice this frame onto what it already holds, or has to
 * ask for the whole transcript instead.
 *
 * A frame is positional, so it is only meaningful against the same conversation
 * and a prefix of the same shape. A receiver holding fewer messages than the
 * frame starts at has a gap; one holding more than the sender has messages the
 * sender no longer does (it compacted, or switched chats), and splicing would
 * render a conversation that never existed.
 */
export function canSpliceFrame({
	baseIndex,
	total,
	localLength,
	onSameChat
}: {
	baseIndex: number
	total: number
	localLength: number
	onSameChat: boolean
}): boolean {
	// A frame that starts at 0 replaces everything, so it needs no prefix to
	// agree with and can be adopted even when it names a different conversation.
	if (baseIndex === 0) return true
	return onSameChat && localLength >= baseIndex && localLength <= total
}
