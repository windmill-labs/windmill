import type { DisplayMessage } from '$lib/components/copilot/chat/shared'

/**
 * Drop the attachment bytes from the transcript a mirror frame carries.
 *
 * A frame goes out several times a second for the whole turn, and the message
 * that holds an image or a pasted file sits in its tail unchanged throughout —
 * so shipping the payloads means re-cloning and re-broadcasting the same
 * megabytes on every tick (the caps allow roughly 16MB of base64 image plus 5MB
 * of file content on a single message). Watchers render the transcript without
 * them and receive the real thing when the turn ends and they re-read the
 * record the driving tab saved.
 */
export function withoutHeavyPayloads(messages: DisplayMessage[]): DisplayMessage[] {
	return messages.map((message) => {
		const images = 'images' in message ? message.images : undefined
		const files = 'files' in message ? message.files : undefined
		const imageUrl = 'imageUrl' in message ? message.imageUrl : undefined
		if (!images?.length && !files?.length && !imageUrl) return message
		const stripped: Record<string, unknown> = { ...message }
		// Dropped rather than blanked: the bubble renders one <img> per entry with
		// no per-image guard, so an emptied dataUrl would show a broken image where
		// the real one is about to appear.
		if (images?.length) delete stripped.images
		// Kept, minus the bytes: the chip is labelled from the name.
		if (files?.length) stripped.files = files.map((f) => ({ ...f, content: '' }))
		// Guarded at the render site, so an empty string renders nothing.
		if (imageUrl) stripped.imageUrl = ''
		return stripped as DisplayMessage
	})
}
