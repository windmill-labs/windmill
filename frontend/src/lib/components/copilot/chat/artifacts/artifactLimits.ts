// What every writer of an artifact has to respect, in one place because they do not share a
// call path. Import-free on purpose: artifactTools pulls ../shared and ../shared pulls
// planMode, so anything shared between them has to sit outside that cycle.

/** Bounds what a snapshot stores and replays to the model. */
export const MAX_ARTIFACT_BYTES = 256 * 1024

/** Returned rather than formatted: the advice for an oversized document is not the advice
 * for an oversized plan, so each caller words its own refusal. */
export function artifactOverflowBytes(content: string): number | undefined {
	const bytes = new TextEncoder().encode(content).length
	return bytes > MAX_ARTIFACT_BYTES ? bytes : undefined
}

/** Truncated rather than rejected: refusing over a cosmetic label would throw away a real
 * content update the model would have to resend. */
const MAX_NOTE_CHARS = 120

export function normalizeChangeNote(note: string | undefined): string | undefined {
	// Blank collapses to undefined, not '': an empty string is not nullish, so it would slip
	// past the picker's fallback and render a row with no label.
	return note?.trim().slice(0, MAX_NOTE_CHARS) || undefined
}
