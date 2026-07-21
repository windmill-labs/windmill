import { describe, it, expect } from 'vitest'
import { attachedTextFileId, uniqueDraftFileName } from './textFileUtils'

describe('attachedTextFileId', () => {
	it('is deterministic and pinned — persisted transcripts depend on this exact value', () => {
		// If the hash implementation changes, every id already persisted in chat
		// history stops matching and old attachments become unreadable. This pins
		// the concrete output, not just self-consistency.
		expect(attachedTextFileId('notes.md', 'hello\n')).toBe(
			attachedTextFileId('notes.md', 'hello\n')
		)
		expect(attachedTextFileId('notes.md', 'hello\n')).toMatchInlineSnapshot(
			`"fxtwc8zefb51jl9s8qn07d"`
		)
	})

	it('differs when the name or the content differs', () => {
		const base = attachedTextFileId('notes.md', 'hello\n')
		expect(attachedTextFileId('notes.md', 'other\n')).not.toBe(base)
		expect(attachedTextFileId('other.md', 'hello\n')).not.toBe(base)
	})
})

describe('uniqueDraftFileName', () => {
	it('suffixes before the extension on a clash, skipping taken suffixes', () => {
		expect(uniqueDraftFileName('notes.md', [])).toBe('notes.md')
		expect(uniqueDraftFileName('notes.md', ['notes.md'])).toBe('notes (2).md')
		expect(uniqueDraftFileName('notes.md', ['notes.md', 'notes (2).md'])).toBe('notes (3).md')
		expect(uniqueDraftFileName('Makefile', ['Makefile'])).toBe('Makefile (2)')
	})
})
