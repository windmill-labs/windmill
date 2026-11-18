import { describe, it, expect } from 'vitest'
import {
	appendPlanModeInstructions,
	derivePlanTitle,
	exitPlanModeRejection,
	isPlanCardTool,
	listBadge,
	listOpenTarget,
	planCardState,
	planVersionTarget,
	planVersionView
} from './planMode'
import { PLAN_MODE_MESSAGES } from './planModeMessages'
import { MAX_ARTIFACT_BYTES } from './artifacts/artifactLimits'

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
		expect(planCardState({ error: PLAN_MODE_MESSAGES.persistenceFailed })).toBeUndefined()
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
			expect(exitPlanModeRejection(args)).toEqual({
				label: PLAN_MODE_MESSAGES.missingSummaryLabel,
				result: PLAN_MODE_MESSAGES.missingSummary
			})
		}
	})

	it('keeps a plan a malformed change_note would otherwise have sunk', () => {
		// `change_note` is optional and cosmetic, and `null` is the shape a model reaches for
		// when omitting an optional field. Reading the summary through a parse of the whole
		// call would fail on it and tell the user there was no plan to approve.
		for (const note of [null, 42, {}]) {
			expect(
				exitPlanModeRejection({ summary: '# Plan\n\nDo it.', change_note: note })
			).toBeUndefined()
		}
	})

	it('refuses a plan too large for the document that has to hold it', () => {
		// The plan goes to the store through the save path, which never sees the artifact
		// tools' cap — so refusing here is the only thing standing between an oversized plan
		// and a card offering to approve one the document silently never received.
		const huge = `# Plan\n\n${'x'.repeat(MAX_ARTIFACT_BYTES)}`
		const rejection = exitPlanModeRejection({ summary: huge })
		expect(rejection?.label).toBe(PLAN_MODE_MESSAGES.oversizedPlanLabel)
		// The model cannot measure bytes, so the refusal has to say how far over it is.
		expect(rejection?.result).toContain(String(MAX_ARTIFACT_BYTES))
	})
})

describe('planVersionView', () => {
	const plan = (approvedVersion: number | undefined, version: number) => ({
		role: 'plan' as const,
		approvedVersion,
		version
	})

	it('reads every version of a plan against the one the user approved', () => {
		// shown is undefined while unpinned, which means the latest.
		const cases: [string, ReturnType<typeof plan>, number | undefined, unknown][] = [
			[
				'never approved, only version',
				plan(undefined, 1),
				undefined,
				{ badge: 'draft', bar: undefined, backToPlan: undefined }
			],
			[
				'never approved, on latest',
				plan(undefined, 3),
				undefined,
				{ badge: 'draft', bar: undefined, backToPlan: undefined }
			],
			[
				'never approved, in history',
				plan(undefined, 3),
				1,
				{ badge: undefined, bar: undefined, backToPlan: undefined }
			],
			[
				'approved is the only version',
				plan(1, 1),
				undefined,
				{ badge: 'plan', bar: undefined, backToPlan: undefined }
			],
			[
				'approved is the latest',
				plan(3, 3),
				undefined,
				{ badge: 'plan', bar: undefined, backToPlan: undefined }
			],
			[
				'unapproved head',
				plan(2, 3),
				undefined,
				{ badge: 'draft', bar: 'unapproved-head', backToPlan: 2 }
			],
			[
				'on the plan, newer draft exists',
				plan(2, 3),
				2,
				{ badge: 'plan', bar: 'approved-with-newer', backToPlan: undefined }
			],
			['behind the plan', plan(2, 3), 1, { badge: undefined, bar: undefined, backToPlan: 2 }],
			[
				// Offering v3 here would pin the current version and re-open it under the very
				// history bar the button exists to leave.
				'behind a plan approved at the head',
				plan(3, 3),
				1,
				{ badge: undefined, bar: undefined, backToPlan: undefined }
			],
			[
				'between the plan and the head',
				plan(1, 4),
				3,
				{ badge: undefined, bar: undefined, backToPlan: 1 }
			]
		]
		for (const [label, artifact, shown, expected] of cases) {
			expect(planVersionView(artifact, shown), label).toEqual(expected)
		}
	})

	it('leaves ordinary artifacts unlabelled at every version', () => {
		expect(planVersionView({ version: 3 }, undefined)).toEqual({
			badge: undefined,
			bar: undefined,
			backToPlan: undefined
		})
		expect(planVersionView({ version: 3 }, 1)).toEqual({
			badge: undefined,
			bar: undefined,
			backToPlan: undefined
		})
	})
})

describe('planVersionTarget', () => {
	it('pins a version only while the document has moved past it', () => {
		// Pinning the current version would open it dressed as history — the stale-version
		// banner over the very text the opener meant to show.
		expect(planVersionTarget({ version: 3 }, 3)).toBe('latest')
		expect(planVersionTarget({ version: 3 }, 2)).toBe(2)
		// Nothing approved, or no document to compare against: there is no version to pin.
		expect(planVersionTarget({ version: 3 }, undefined)).toBe('latest')
		expect(planVersionTarget(undefined, 2)).toBe('latest')
		// A single-version document has `version` unset.
		expect(planVersionTarget({}, 1)).toBe('latest')
	})
})

describe('listOpenTarget', () => {
	it('names a version for a plan and none for an ordinary artifact', () => {
		// `'latest'` clears the tab's pin; only omitting it keeps the reader where they were,
		// and an ordinary artifact's version is theirs to choose.
		expect(listOpenTarget({ version: 3 })).toBeUndefined()
		expect(listOpenTarget({ version: 3, approvedVersion: 2 })).toBeUndefined()
		expect(listOpenTarget({ role: 'plan', version: 3, approvedVersion: 2 })).toBe(2)
		expect(listOpenTarget({ role: 'plan', version: 3, approvedVersion: 3 })).toBe('latest')
		expect(listOpenTarget({ role: 'plan', version: 3 })).toBe('latest')
	})
})

describe('listBadge', () => {
	it('labels the version the row opens, not the newest', () => {
		// The head is an unapproved draft, but the row opens v2, so labelling the head would
		// promise a draft and hand over the plan.
		expect(listBadge({ role: 'plan', version: 3, approvedVersion: 2 })).toBe('plan')
		expect(listBadge({ role: 'plan', version: 3 })).toBe('draft')
		// No pill at all, so the row keeps showing the artifact's kind.
		expect(listBadge({ version: 3, approvedVersion: 2 })).toBeUndefined()
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
