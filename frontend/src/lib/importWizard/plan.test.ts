import { describe, expect, it } from 'vitest'

import {
	planProblem,
	planToSearch,
	planWorkspaceId,
	readPlan,
	type ImportPlan,
	type WizardStep
} from './plan'

// The wizard keeps no state of its own — the plan *is* the URL. Back, forward, the
// stepper and a pasted link are all the same operation, which only holds if this
// round trip is lossless.

const read = (search: string) => readPlan(new URL(`https://x.dev/projects/import${search}`))
const roundTrip = (plan: ImportPlan, step: WizardStep) => read(planToSearch(plan, step))

describe('readPlan / planToSearch round trip', () => {
	const cases: Array<[string, ImportPlan, WizardStep]> = [
		['bare slug', { slug: 'support-automation' }, 1],
		['new destination', { slug: 's', destination: { kind: 'new', id: 'w', name: 'W' } }, 2],
		[
			'new with username',
			{ slug: 's', destination: { kind: 'new', id: 'w', name: 'W', username: 'ada' } },
			2
		],
		[
			'existing destination',
			{ slug: 's', destination: { kind: 'existing', workspaceId: 'admins' } },
			3
		],
		[
			'folder carried',
			{ slug: 's', destination: { kind: 'existing', workspaceId: 'admins' }, folder: 'finance' },
			3
		]
	]

	for (const [name, plan, step] of cases) {
		it(name, () => {
			expect(roundTrip(plan, step)).toEqual({ plan, step })
		})
	}

	it('survives characters that need encoding', () => {
		const plan: ImportPlan = {
			slug: 'a b&c=d',
			destination: { kind: 'new', id: 'w', name: 'Name & Co = 100%' }
		}
		expect(roundTrip(plan, 2)).toEqual({ plan, step: 2 })
	})
})

describe('readPlan', () => {
	it('reads legacy links that carry only a workspace', () => {
		expect(read('?hub=s&workspace=admins').plan.destination).toEqual({
			kind: 'existing',
			workspaceId: 'admins'
		})
	})

	it('reads legacy links that carry only new_workspace_id, falling the name back to the id', () => {
		expect(read('?hub=s&new_workspace_id=w').plan.destination).toEqual({
			kind: 'new',
			id: 'w',
			name: 'w',
			username: undefined
		})
	})

	it('clamps and rounds the step', () => {
		expect(read('?hub=s&step=0').step).toBe(1)
		// 4 is the optional setup step, so that is the ceiling.
		expect(read('?hub=s&step=9').step).toBe(4)
		expect(read('?hub=s&step=nope').step).toBe(1)
		// A fractional step would match neither `=== 2` nor `=== 3`.
		expect(read('?hub=s&step=2.5').step).toBe(3)
		expect(read('?hub=s&step=2.4').step).toBe(2)
	})

	it('has no destination when nothing names one', () => {
		expect(read('?hub=s').plan.destination).toBeUndefined()
	})
})

/**
 * `create_workspace` takes the username it is given and writes it to `usr.username` without
 * checking it — `Some("")` passes its only guard — so a blank or malformed one becomes a
 * workspace whose owner has no usable name. The wizard is the last thing that can refuse it.
 */
describe('planProblem — the new-workspace username', () => {
	const dest = (username?: string) => ({
		slug: 'calendly',
		destination: { kind: 'new' as const, id: 'calendly', name: 'Calendly', username }
	})

	it('asks for nothing when the instance derives the username', () => {
		expect(planProblem(dest(undefined))).toBeUndefined()
	})

	it('refuses a blank or whitespace-only username', () => {
		expect(planProblem(dest(''))).toMatch(/needs a username/i)
		expect(planProblem(dest('   '))).toMatch(/needs a username/i)
	})

	it('refuses one the backend would store verbatim but never accept elsewhere', () => {
		expect(planProblem(dest('1bad'))).toMatch(/letters and numbers/i)
		expect(planProblem(dest('a b'))).toMatch(/letters and numbers/i)
	})

	it('accepts a valid one', () => {
		expect(planProblem(dest('guilhem'))).toBeUndefined()
	})
})

/** `validate_workspace_name` refuses > 50 chars, and only at creation — two steps later. */
describe('planProblem — the new-workspace name length', () => {
	const named = (name: string) => ({
		slug: 'calendly',
		destination: { kind: 'new' as const, id: 'calendly', name }
	})

	it('accepts the longest name the backend takes', () => {
		expect(planProblem(named('x'.repeat(50)))).toBeUndefined()
	})

	it('refuses one character more, rather than failing at create', () => {
		expect(planProblem(named('x'.repeat(51)))).toMatch(/too long/i)
	})
})

describe('planProblem', () => {
	it('names what is missing, in the order the wizard asks for it', () => {
		expect(planProblem({ slug: '' })).toMatch(/No project/)
		expect(planProblem({ slug: 's' })).toMatch(/Pick a destination/)
		expect(planProblem({ slug: 's', destination: { kind: 'new', id: 'w', name: '' } })).toMatch(
			/needs a name/
		)
		expect(planProblem({ slug: 's', destination: { kind: 'existing' } })).toMatch(
			/Pick the workspace/
		)
	})

	it('validates the id of either destination kind', () => {
		expect(
			planProblem({ slug: 's', destination: { kind: 'new', id: 'not valid', name: 'W' } })
		).toMatch(/letters, numbers and dashes/)
		// An existing id arrives from the URL just as a new one does.
		expect(
			planProblem({ slug: 's', destination: { kind: 'existing', workspaceId: '../admins' } })
		).toMatch(/not a valid workspace id/)
	})

	it('rejects a folder name the import could not create', () => {
		const base: ImportPlan = { slug: 's', destination: { kind: 'existing', workspaceId: 'admins' } }
		expect(planProblem({ ...base, folder: 'ok_folder-1' })).toBeUndefined()
		expect(planProblem({ ...base, folder: 'not ok' })).toMatch(/Folder/)
	})
})

describe('planWorkspaceId', () => {
	it('is the id either kind of destination will end up in', () => {
		expect(planWorkspaceId({ slug: 's', destination: { kind: 'new', id: 'w', name: 'W' } })).toBe(
			'w'
		)
		expect(
			planWorkspaceId({ slug: 's', destination: { kind: 'existing', workspaceId: 'admins' } })
		).toBe('admins')
		expect(planWorkspaceId({ slug: 's' })).toBeUndefined()
	})
})
