import { describe, it, expect } from 'vitest'
import { flowDraftSig } from './flowDraftSig'

describe('flowDraftSig', () => {
	it('changes when only the summary changes (the regression this guards)', () => {
		const base = { value: { modules: [] }, schema: {}, summary: 'a' }
		const renamed = { ...base, summary: 'b' }
		expect(flowDraftSig(base)).not.toBe(flowDraftSig(renamed))
	})

	it('is stable for equal value/schema/summary', () => {
		const a = { value: { modules: [1] }, schema: { x: 1 }, summary: 's' }
		const b = { value: { modules: [1] }, schema: { x: 1 }, summary: 's' }
		expect(flowDraftSig(a)).toBe(flowDraftSig(b))
	})

	it('changes when the value changes', () => {
		expect(flowDraftSig({ value: { modules: [] }, summary: 's' })).not.toBe(
			flowDraftSig({ value: { modules: [1] }, summary: 's' })
		)
	})

	it('changes when only draft_path changes (path rename triggers a save)', () => {
		const base = { value: { modules: [] }, summary: 's', draft_path: 'u/admin/draft_abc' }
		const renamed = { ...base, draft_path: 'u/admin/friendly' }
		expect(flowDraftSig(base)).not.toBe(flowDraftSig(renamed))
	})

	it('changes when only path changes', () => {
		const base = { value: { modules: [] }, summary: 's', path: 'u/admin/a' }
		expect(flowDraftSig(base)).not.toBe(flowDraftSig({ ...base, path: 'u/admin/b' }))
	})
})
