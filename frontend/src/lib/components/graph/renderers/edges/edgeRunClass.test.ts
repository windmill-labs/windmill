import { describe, it, expect } from 'vitest'
import { edgeRunClass } from './edgeRunClass'
import type { FlowRunStatus } from '../../flowRunStatus.svelte'

const status = (notRun: string[] = []) =>
	({
		showsRun: true,
		didNotRun: (id: string) => notRun.includes(id)
	}) as unknown as FlowRunStatus

describe('edgeRunClass', () => {
	it("draws a handler's edge plain, even next to a step that did not run", () => {
		expect(edgeRunClass(status(['a']), { handler: true, steps: ['a'] })).toBe('')
	})

	it('hides a replaced exit and fades a bypassed one or one through a step that did not run', () => {
		expect(edgeRunClass(status(), { bypassed: 'replaced' })).toBe('hidden')
		expect(edgeRunClass(status(), { bypassed: true })).toContain('dasharray')
		expect(edgeRunClass(status(['b']), { steps: ['a', 'b'] })).toContain('dasharray')
		expect(edgeRunClass(status(), { steps: ['a', 'b'] })).toBe('')
	})
})
