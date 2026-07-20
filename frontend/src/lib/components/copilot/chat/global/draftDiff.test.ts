import { describe, expect, it } from 'vitest'
import { draftDeployedPatch } from './draftDiff'

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
