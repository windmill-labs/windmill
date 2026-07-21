import { describe, it, expect } from 'vitest'
import { attachedTextFileId, foldIntoDraft, uniqueDraftFileName } from './textFileUtils'

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

describe('foldIntoDraft', () => {
	// Attach batches overlap (each awaits its file reads), so normalization runs
	// against the list as it stands at commit — these pin the fold semantics.
	it('drops identical duplicates against the live list and within the batch', () => {
		const current = [{ name: 'a.md', content: 'x', id: attachedTextFileId('a.md', 'x') }]
		const commit = foldIntoDraft(current, [
			{ name: 'a.md', content: 'x' },
			{ name: 'b.md', content: 'y' },
			{ name: 'b.md', content: 'y' }
		])
		expect(commit.map((f) => f.name)).toEqual(['b.md'])
	})

	it('renames a same-name clash committed by another batch and mints the id from the final name', () => {
		const current = [{ name: 'a.md', content: 'old', id: attachedTextFileId('a.md', 'old') }]
		const commit = foldIntoDraft(current, [{ name: 'a.md', content: 'new' }])
		expect(commit).toEqual([
			{ name: 'a (2).md', content: 'new', id: attachedTextFileId('a (2).md', 'new') }
		])
	})

	it('dedupes an identical re-drop even after its first copy was courtesy-renamed', () => {
		// A name clash renames the first copy (notes.md → notes (2).md), erasing the
		// original name — an identical re-drop must still read as a duplicate, in the
		// same batch and in a later overlapping one.
		const current = [
			{ name: 'notes.md', content: 'old', id: attachedTextFileId('notes.md', 'old') }
		]
		const first = foldIntoDraft(current, [
			{ name: 'notes.md', content: 'new' },
			{ name: 'notes.md', content: 'new' }
		])
		expect(first.map((f) => f.name)).toEqual(['notes (2).md'])

		const second = foldIntoDraft([...current, ...first], [{ name: 'notes.md', content: 'new' }])
		expect(second).toEqual([])
	})
})
