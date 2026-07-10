/**
 * Big-paste collapsing for the AI chat input.
 *
 * When a user pastes a large block into a `ContextTextarea`, instead of dumping
 * every line into the input we register the blob and insert a compact *token*
 * into the text. The token's visible characters are exactly the chip label, so
 * the transparent textarea text and the highlight overlay stay the same width
 * (caret alignment is preserved). A run of zero-width characters appended to the
 * label encodes the attachment id (its length == the id) so duplicate labels map
 * back unambiguously while staying invisible and zero-width in both layers.
 *
 * The token lives in the message text; on send the tokens are expanded back to
 * the full content for the LLM, while the displayed message keeps the tokens +
 * a `PasteAttachment[]` registry so the chip can render and toggle everywhere.
 */

export type PasteAttachment = {
	id: number
	lines: number
	content: string
}

/** Zero-width space; `id` copies are appended after the label to encode the id. */
const ZW = String.fromCharCode(0x200b)

/** Collapse a paste when it has more than this many lines… */
export const PASTE_LINE_THRESHOLD = 10
/** …or more than this many characters (catches giant single-line blobs). */
export const PASTE_CHAR_THRESHOLD = 1000

/** Number of display lines, ignoring a single trailing newline — line-based
 *  copies usually include one, which would otherwise inflate the count by one
 *  (e.g. a 10-line selection counting as 11). */
export function countLines(text: string): number {
	return (text.endsWith('\n') ? text.slice(0, -1) : text).split('\n').length
}

export function shouldCollapsePaste(text: string): boolean {
	return countLines(text) > PASTE_LINE_THRESHOLD || text.length > PASTE_CHAR_THRESHOLD
}

/** "1 line" / "N lines" — the single source of line-count pluralization. */
export function lineCountLabel(lines: number): string {
	return `${lines} ${lines === 1 ? 'line' : 'lines'}`
}

export function pasteLabel(lines: number): string {
	return `Pasted ${lineCountLabel(lines)} · click to expand`
}

/** The text inserted into the input for a collapsed paste. */
export function makePasteToken(att: PasteAttachment): string {
	return pasteLabel(att.lines) + ZW.repeat(att.id)
}

/**
 * Fresh regex each call — the global flag carries `lastIndex` state, so a shared
 * instance would desync across `replace`/`matchAll`/`exec` callers. Group 1 is
 * the zero-width run whose length is the attachment id.
 */
export function pasteTokenRegex(): RegExp {
	return new RegExp(`Pasted \\d+ lines? · click to expand(${ZW}+)`, 'gu')
}

export function nextPasteId(pastes: PasteAttachment[]): number {
	return pastes.reduce((max, p) => Math.max(max, p.id), 0) + 1
}

/** Replace every recognized token with its full content (for the LLM). */
export function expandPasteTokens(text: string, pastes: PasteAttachment[] | undefined): string {
	if (!pastes?.length) return text
	return text.replace(pasteTokenRegex(), (match, zw: string) => {
		const att = pastes.find((p) => p.id === zw.length)
		return att ? att.content : match
	})
}

export type PasteSegment = { type: 'text'; value: string } | { type: 'paste'; att: PasteAttachment }

/** Split text into plain-text and paste-chip segments (for rendering). */
export function splitPasteTokens(
	text: string,
	pastes: PasteAttachment[] | undefined
): PasteSegment[] {
	if (!pastes?.length) return text ? [{ type: 'text', value: text }] : []
	const segments: PasteSegment[] = []
	let last = 0
	for (const m of text.matchAll(pasteTokenRegex())) {
		const att = pastes.find((p) => p.id === m[1].length)
		if (!att || m.index === undefined) continue
		if (m.index > last) segments.push({ type: 'text', value: text.slice(last, m.index) })
		segments.push({ type: 'paste', att })
		last = m.index + m[0].length
	}
	if (last < text.length) segments.push({ type: 'text', value: text.slice(last) })
	return segments
}
