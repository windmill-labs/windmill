import { describe, it, expect } from 'vitest'
import {
	appendPlanModeInstructions,
	exitPlanModeRejection,
	isPlanCardTool,
	PLAN_MODE_MESSAGES
} from './planMode'

describe('isPlanCardTool', () => {
	it('rejects inherited property names', () => {
		expect(isPlanCardTool('exit_plan_mode')).toBe(true)
		expect(isPlanCardTool('enter_plan_mode')).toBe(true)
		// Tool names come from the model, so `in` would render these as plan cards.
		expect(isPlanCardTool('toString')).toBe(false)
		expect(isPlanCardTool('constructor')).toBe(false)
		expect(isPlanCardTool('__proto__')).toBe(false)
		expect(isPlanCardTool(undefined)).toBe(false)
	})
})

describe('exitPlanModeRejection', () => {
	it('passes a real plan and refuses anything with nothing to approve', () => {
		expect(exitPlanModeRejection({ summary: '# Plan\n\nDo it.' })).toBeUndefined()
		for (const args of [{}, { summary: '' }, { summary: '  \n ' }, { summary: 42 }, null]) {
			expect(exitPlanModeRejection(args)).toBe(PLAN_MODE_MESSAGES.missingSummary)
		}
	})
})

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
