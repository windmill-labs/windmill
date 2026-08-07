import { describe, it, expect } from 'vitest'
import {
	appendPlanModeInstructions,
	derivePlanTitle,
	exitPlanModeRejection,
	isPlanCardTool,
	planCardState,
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

describe('planCardState', () => {
	it('reads as declined only when the user decided, not on any error', () => {
		expect(
			planCardState({ error: 'Tool execution was cancelled by user', declinedByUser: true })
		).toBe('declined')
		// Everything else that ends in an error renders as an ordinary tool error: claiming a
		// decision the user never made is the whole failure mode this guards.
		expect(planCardState({ error: PLAN_MODE_MESSAGES.exitOutsidePlanMode })).toBeUndefined()
		expect(
			planCardState({ error: 'Tool call arguments were invalid or truncated' })
		).toBeUndefined()
		expect(planCardState({ error: 'Unknown tool call: enter_plan_mode.' })).toBeUndefined()
	})

	it('holds a call that has not resolved yet at pending', () => {
		expect(planCardState({ needsConfirmation: true })).toBe('pending')
		expect(planCardState({ isLoading: true })).toBe('pending')
		// A card waiting its turn behind another tool has no error and no confirmation
		// pending yet, so without this it would read as already approved.
		expect(planCardState({ isQueued: true })).toBe('pending')
		expect(planCardState({ isStreamingArguments: true })).toBe('pending')
		expect(planCardState({})).toBe('settled')
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

describe('derivePlanTitle', () => {
	it('uses the first markdown heading of any level', () => {
		expect(derivePlanTitle('## Add a retry policy\n\nSteps...')).toBe('Add a retry policy')
		expect(derivePlanTitle('Lead-in\n\n# Top level\n\n## Later')).toBe('Top level')
	})

	it('falls back to a default when the summary has no heading', () => {
		expect(derivePlanTitle('Just prose, no heading.')).toBe('Implementation plan')
		expect(derivePlanTitle('#### Too deep')).toBe('Implementation plan')
		// A bare '#' must not swallow the blank line and title the plan after the next prose.
		expect(derivePlanTitle('#\n\nJust prose.')).toBe('Implementation plan')
	})

	it('ignores headings inside fenced code blocks', () => {
		expect(derivePlanTitle('Lead-in.\n\n```bash\n# Install the deps\n```\n\n## Real title')).toBe(
			'Real title'
		)
		expect(derivePlanTitle('Lead-in.\n\n~~~bash\n# Install the deps\n~~~\n\n## Real title')).toBe(
			'Real title'
		)
		// A longer fence closes only on its own length, so an inner fence must not end it.
		expect(derivePlanTitle('Lead-in.\n\n````md\n```\n# Inner\n```\n````\n\n## Real title')).toBe(
			'Real title'
		)
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
