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
