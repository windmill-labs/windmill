import { describe, it, expect } from 'vitest'
import { appendPlanModeInstructions } from './planMode'

describe('appendPlanModeInstructions', () => {
	const base = { role: 'system' as const, content: 'BASE' }

	it('appends the plan-mode block below the base prompt', () => {
		const result = appendPlanModeInstructions(base, 0)
		expect(typeof result.content).toBe('string')
		expect(result.content).toMatch(/^BASE\n\n/)
		expect(result.content).toContain('Plan mode active')
	})

	it('does not append the escalation steer below the threshold', () => {
		expect(appendPlanModeInstructions(base, 2).content).not.toContain('STOP retrying tools')
	})

	it('appends the escalation steer at or above the threshold', () => {
		expect(appendPlanModeInstructions(base, 3).content).toContain('STOP retrying tools')
	})

	it('passes non-string content through unchanged', () => {
		const arrayContent = { role: 'system' as const, content: [{ type: 'text', text: 'x' }] as any }
		expect(appendPlanModeInstructions(arrayContent, 5)).toBe(arrayContent)
	})
})
