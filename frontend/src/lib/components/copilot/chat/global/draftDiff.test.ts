import { describe, expect, it } from 'vitest'
import { changedLineIndices, draftDeployedPatch, textFilePatch } from './draftDiff'

describe('draftDeployedPatch', () => {
	it('returns an empty string for identical values', () => {
		expect(draftDeployedPatch({ a: 1, b: 'x' }, { a: 1, b: 'x' })).toBe('')
	})

	it('ignores key-order differences', () => {
		expect(draftDeployedPatch({ a: 1, b: 2 }, { b: 2, a: 1 })).toBe('')
	})

	it('treats null fields and absent fields as equal', () => {
		expect(draftDeployedPatch({ a: 1, tag: null }, { a: 1 })).toBe('')
		// but a real value vs null/absent still diffs
		expect(draftDeployedPatch({ a: 1, tag: 'prod' }, { a: 1 })).toContain('-tag: prod')
	})

	it('diffs multiline code fields line-by-line', () => {
		const deployed = { content: 'line1\nline2\nline3\n', language: 'bun' }
		const draft = { content: 'line1\nchanged\nline3\n', language: 'bun' }
		const patch = draftDeployedPatch(deployed, draft)
		expect(patch).toContain('-  line2')
		expect(patch).toContain('+  changed')
		// Unchanged lines are context, not part of the change
		expect(patch).not.toContain('-  line1')
	})

	it('renders a whole draft as additions when there is no deployed side', () => {
		const patch = draftDeployedPatch(undefined, { summary: 'new item', value: { modules: [] } })
		expect(patch).toContain('+summary: new item')
		expect(patch).not.toMatch(/^-[^-]/m)
	})
})

describe('windowPatch', () => {
	it('continues from the last delivered line when the char cap cuts a window', async () => {
		const { windowPatch } = await import('./draftDiff')
		const patch = Array.from({ length: 20 }, (_, i) => `line-${String(i).padStart(2, '0')}`).join(
			'\n'
		)
		// Cap fits ~5 complete 7-char lines ("line-NN" + newline).
		const out = windowPatch(patch, 0, 20, 40)
		const continueAt = Number(out.match(/offset=(\d+)/)?.[1])
		expect(continueAt).toBeGreaterThan(0)
		expect(continueAt).toBeLessThan(20)
		// The continuation must resume exactly at the first undelivered line.
		const delivered = out.split('\n').filter((l) => l.startsWith('line-'))
		expect(delivered[delivered.length - 1]).toBe(`line-${String(continueAt - 1).padStart(2, '0')}`)
		const next = windowPatch(patch, continueAt, 20, 40)
		expect(next).toContain(`line-${String(continueAt).padStart(2, '0')}`)
	})

	it('reports an overshooting offset instead of an impossible range', async () => {
		const { windowPatch } = await import('./draftDiff')
		expect(windowPatch('a\nb', 10, 5, 100)).toContain('no lines at offset 10')
	})

	it('steps past a single line larger than the whole budget', async () => {
		const { windowPatch } = await import('./draftDiff')
		const patch = ['x'.repeat(100), 'after'].join('\n')
		const out = windowPatch(patch, 0, 10, 40)
		expect(out).toContain('offset=1')
		const next = windowPatch(patch, 1, 10, 40)
		expect(next).toContain('after')
	})
})

describe('changedLineIndices', () => {
	it('counts source lines starting with ++ or -- but never the file labels', () => {
		const patch = textFilePatch('a\n--counter\nb\n', 'a\n++counter\nb\n', 'deployed', 'draft')
		const lines = patch.split('\n')
		const changed = changedLineIndices(patch).map((i) => lines[i])
		// A changed `--counter`/`++counter` source line is byte-identical to a
		// file label prefix — only hunk-awareness keeps it.
		expect(changed).toContain('---counter')
		expect(changed).toContain('+++counter')
		// The actual file labels never count as changes.
		expect(changed.some((l) => l.includes('deployed') || l.includes('draft'))).toBe(false)
	})
})
