import { describe, expect, it } from 'vitest'
import { contextElementKey, isSameContextElement, type ContextElement } from './context'

describe('mentioned workspace item identity', () => {
	it('tells apart two drafts titled by the same draft path', () => {
		const a: ContextElement = { type: 'workspace_flow', path: 'u/me/draft_a', title: 'u/me/report' }
		const b: ContextElement = { type: 'workspace_flow', path: 'u/me/draft_b', title: 'u/me/report' }
		expect(isSameContextElement(a, b)).toBe(false)
		expect(contextElementKey(a)).not.toBe(contextElementKey(b))
	})
})
