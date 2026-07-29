import { describe, it, expect, vi } from 'vitest'
import { newDraftTriggerPath } from './utils'

vi.mock('$lib/stores', () => ({
	userStore: { subscribe: (run: any) => (run(undefined), () => {}) }
}))

describe('newDraftTriggerPath', () => {
	it('gives the primary schedule the runnable path, so it deploys as the primary', () => {
		expect(newDraftTriggerPath('f/team/my_flow', 'schedule', [], true)).toBe('f/team/my_flow')
	})

	it('suffixes by kind for everything else', () => {
		expect(newDraftTriggerPath('f/team/my_flow', 'http', [])).toBe('f/team/my_flow_http')
		expect(newDraftTriggerPath('f/team/my_flow', 'schedule', [], false)).toBe(
			'f/team/my_flow_schedule'
		)
	})

	it('never reuses a path already taken by another trigger of the kind', () => {
		const first = newDraftTriggerPath('f/team/my_flow', 'http', [])
		const second = newDraftTriggerPath('f/team/my_flow', 'http', [first])
		const third = newDraftTriggerPath('f/team/my_flow', 'http', [first, second])
		expect(new Set([first, second, third]).size).toBe(3)
		expect(second.startsWith('f/team/my_flow_http_')).toBe(true)
	})

	it('falls back off the runnable path when the primary schedule slot is taken', () => {
		expect(newDraftTriggerPath('f/team/my_flow', 'schedule', ['f/team/my_flow'], true)).toBe(
			'f/team/my_flow_schedule'
		)
	})
})
