import { describe, it, expect } from 'vitest'
import { edgeRunClass } from './edgeRunClass'
import type { FlowRunStatus } from '../../flowRunStatus.svelte'

const status = (tone: 'failure' | 'recovered' | undefined, notRun: string[] = []) =>
	({
		showsRun: true,
		handlerEdgeTone: () => tone,
		didNotRun: (id: string) => notRun.includes(id)
	}) as unknown as FlowRunStatus

describe('edgeRunClass', () => {
	it("paints a handler's edge in its tone, even next to a step that did not run", () => {
		const cls = edgeRunClass(status('recovered', ['a']), {
			handler: { id: 'failure', side: 'out' },
			steps: ['a']
		})
		expect(cls).toContain('stroke-green-400')
		expect(cls).not.toContain('dasharray')
	})

	it('hides a replaced exit and fades a bypassed one or one through a step that did not run', () => {
		expect(edgeRunClass(status(undefined), { bypassed: 'replaced' })).toBe('hidden')
		expect(edgeRunClass(status(undefined), { bypassed: true })).toContain('dasharray')
		expect(edgeRunClass(status(undefined, ['b']), { steps: ['a', 'b'] })).toContain('dasharray')
		expect(edgeRunClass(status(undefined), { steps: ['a', 'b'] })).toBe('')
	})
})
