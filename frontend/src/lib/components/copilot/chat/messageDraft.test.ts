import { describe, it, expect } from 'vitest'
import { MessageDraft } from './messageDraft.svelte'

// Fold/rename/dedupe semantics are pinned in textFileUtils.test.ts; these pin
// the draft-level guarantees: lanes move together, restores respect occupancy,
// aggregation always applies the rules.

describe('MessageDraft', () => {
	it('take() snapshots and clears all four lanes atomically', () => {
		const d = new MessageDraft({
			text: 'hello',
			pastes: [{ id: 'p1', content: 'x' } as any],
			images: [{ dataUrl: 'i1' } as any],
			files: [{ name: 'a.md', content: 'a' }]
		})
		const snap = d.take()
		expect(snap.text).toBe('hello')
		expect(snap.pastes).toHaveLength(1)
		expect(snap.images).toHaveLength(1)
		expect(snap.files).toHaveLength(1)
		expect(d.isEmpty).toBe(true)
	})

	it('replaceIfEmpty declines when any lane is occupied', () => {
		const d = new MessageDraft({ files: [{ name: 'a.md', content: 'a' }] })
		expect(d.replaceIfEmpty({ text: 'restored' })).toBe(false)
		expect(d.files).toHaveLength(1)
		d.clear()
		expect(d.replaceIfEmpty({ text: 'restored' })).toBe(true)
		expect(d.text).toBe('restored')
	})

	it('prepend puts restored text above the current draft and folds files in', () => {
		const d = new MessageDraft({ text: 'typing', files: [{ name: 'a.md', content: 'a' }] })
		const res = d.prepend({
			text: 'restored',
			files: [
				{ name: 'a.md', content: 'a' }, // identical → dedupes
				{ name: 'a.md', content: 'b' } // clash → courtesy rename
			]
		})
		expect(res.mergedIntoDraft).toBe(true)
		expect(d.text).toBe('restored\n\ntyping')
		expect(d.files.map((f) => f.name)).toEqual(['a.md', 'a (2).md'])
	})

	it('addFiles reports drops at the slot cap and the byte budget', () => {
		const d = new MessageDraft()
		const many = Array.from({ length: 10 }, (_, i) => ({ name: `${i}.md`, content: `${i}` }))
		expect(d.addFiles(many).droppedAtCap).toBe(2)
		expect(d.files).toHaveLength(8)

		const e = new MessageDraft()
		// Budget admits by decoded size AFTER the fold — the identical duplicate is
		// deduped, not charged.
		const res = e.addFiles(
			[
				{ name: 'a.md', content: 'aaaa' },
				{ name: 'a.md', content: 'aaaa' },
				{ name: 'b.md', content: 'bbbb' },
				{ name: 'c.md', content: 'cccc' }
			],
			8
		)
		expect(e.files.map((f) => f.name)).toEqual(['a.md', 'b.md'])
		expect(res.droppedAtBudget).toBe(1)
	})
})
