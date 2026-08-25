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

/** Emptied outright; guarded at the render site, so it renders nothing. */
const BLANKED_FIELDS = ['imageUrl'] as const

function isHeavy(message: DisplayMessage): boolean {
	const m = message as Record<string, any>
	return (
		DROPPED_LIST_FIELDS.some((f) => m[f]?.length) ||
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
		for (const [f, item] of BLANKED_ITEM_FIELDS) {
			if (stripped[f]?.length) stripped[f] = stripped[f].map((v: any) => ({ ...v, [item]: '' }))
		}
		for (const f of BLANKED_FIELDS) if (stripped[f]) stripped[f] = ''
		return stripped as DisplayMessage
	})
}

/**
 * Put back the payloads a watcher already holds.
 *
 * A frame carries the last several messages, but only the newest one or two are
 * ever new to the watcher — the rest it loaded from IndexedDB with attachments
 * intact. Overwriting those with the stripped copies makes a screenshot from an
 * earlier turn vanish the moment another tab starts a turn, and stay gone until
 * the turn ends. Positional, which is what the frame's own indexing already
 * assumes: a transcript only grows within a turn, and a rewrite (compaction)
 * forces a full frame instead of a tail.
 */
export function withRestoredPayloads(
	incoming: DisplayMessage[],
	localAt: (offset: number) => DisplayMessage | undefined
): DisplayMessage[] {
	return incoming.map((message, i) => {
		const local = localAt(i) as Record<string, any> | undefined
		if (!local || local.role !== (message as any).role) return message
		const merged: Record<string, any> = { ...message }
		let changed = false
		for (const f of DROPPED_LIST_FIELDS) {
			if (merged[f] === undefined && local[f]?.length) {
				merged[f] = local[f]
				changed = true
			}
		}
		for (const [f, item] of BLANKED_ITEM_FIELDS) {
			if (!merged[f]?.length || !local[f]?.length) continue
			merged[f] = merged[f].map((v: any, j: number) =>
				v[item] === '' && local[f][j]?.[item] ? { ...v, [item]: local[f][j][item] } : v
			)
			changed = true
		}
		for (const f of BLANKED_FIELDS) {
			if (merged[f] === '' && local[f]) {
				merged[f] = local[f]
				changed = true
			}
		}
		return (changed ? merged : message) as DisplayMessage
	})
}
