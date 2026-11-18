import {
	type PasteAttachment,
	type PasteSegment,
	expandPasteTokens,
	splitPasteTokens
} from './pasteTokens'

/**
 * A composed-but-not-yet-sent chat message: the editable text plus the
 * collapsed-paste registry its tokens point into (see pasteTokens.ts).
 *
 * The two travel together because paste tokens live *inside* the text — read it
 * through {@link expanded} (for the model / title) or {@link segments} (for chip
 * rendering) so no caller can leak raw tokens by forgetting to expand.
 *
 * Deliberately excludes `selectedContext` (the `@`-mention registry): that is
 * owned by ContextManager and travels *beside* the text, not inside it.
 */
export type ChatDraft = { text: string; pastes: PasteAttachment[] }

export function chatDraft(text: string, pastes?: PasteAttachment[]): ChatDraft {
	return { text, pastes: pastes ?? [] }
}

/** Build a draft from a stored message. Structural param (not a DisplayMessage
 *  import) to keep this module free of a cycle with shared.ts. */
export function messageDraft(m: { content: string; pastes?: PasteAttachment[] }): ChatDraft {
	return chatDraft(m.content, m.pastes)
}

/**
 * The ONLY way to read the model-bound / flattened text: paste tokens expanded
 * to their full content. Used by the LLM prepare* path, the inline ⌘K path, and
 * the saved-chat title.
 */
export function expanded(d: ChatDraft): string {
	return expandPasteTokens(d.text, d.pastes)
}

/** Text / paste-chip segments, for rendering the draft (input overlay, bubble). */
export function segments(d: ChatDraft): PasteSegment[] {
	return splitPasteTokens(d.text, d.pastes)
}
