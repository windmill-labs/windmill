import { describe, it, expect } from 'vitest'
import { appendPlanModeInstructions, derivePlanTitle } from './planMode'

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
