import { describe, expect, it } from 'vitest'
import { chatDraft, expanded, messageDraft, segments } from './chatDraft'
import { type PasteAttachment, makePasteToken } from './pasteTokens'

const att = (id: number, lines: number, content: string): PasteAttachment => ({
	id,
	lines,
	content
})

describe('expanded', () => {
	it('expands paste tokens to their full content', () => {
		const a = att(1, 12, 'line1\nline2')
		const d = chatDraft(`before ${makePasteToken(a)} after`, [a])
		expect(expanded(d)).toBe('before line1\nline2 after')
	})

	it('is a no-op with no pastes', () => {
		expect(expanded(chatDraft('plain text'))).toBe('plain text')
		expect(expanded(chatDraft('plain text', []))).toBe('plain text')
	})
})

describe('segments', () => {
	it('splits into text and paste segments in order', () => {
		const a = att(1, 12, 'AAA')
		expect(segments(chatDraft(`hi ${makePasteToken(a)} bye`, [a]))).toEqual([
			{ type: 'text', value: 'hi ' },
			{ type: 'paste', att: a },
			{ type: 'text', value: ' bye' }
		])
	})

	it('returns a single text segment with no pastes', () => {
		expect(segments(chatDraft('plain'))).toEqual([{ type: 'text', value: 'plain' }])
	})
})

describe('messageDraft', () => {
	it('builds from a {content, pastes} message', () => {
		const a = att(1, 12, 'AAA')
		const token = makePasteToken(a)
		const d = messageDraft({ content: `x ${token}`, pastes: [a] })
		expect(expanded(d)).toBe('x AAA')
	})

	it('tolerates a message with no pastes (e.g. non-user roles)', () => {
		const d = messageDraft({ content: 'just text' })
		expect(d.pastes).toEqual([])
		expect(expanded(d)).toBe('just text')
	})
})
