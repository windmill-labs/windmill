import { describe, expect, it } from 'vitest'
import { WorkingScope } from './workingScope'

const alice = { username: 'alice', folders: ['crm', 'crm_analytics'] }

describe('WorkingScope.outsidePath', () => {
	const scope = new WorkingScope(['f/crm'], alice)

	it('names a write outside the scope, a trigger request included', () => {
		expect(scope.outsidePath({ path: 'f/crm_analytics/weekly' })).toBe('f/crm_analytics/weekly')
		expect(scope.outsidePath({ kind: 'http', config: { path: 'f/ops/hook' } })).toBe('f/ops/hook')
	})

	it('lets through writes inside it, paths in a payload, and the runnable a schedule runs', () => {
		expect(scope.outsidePath({ path: 'f/crm/sync' })).toBeUndefined()
		expect(scope.outsidePath({ path: 'f/crm/app', value: { path: 'f/ops/x' } })).toBeUndefined()
		expect(scope.outsidePath({ path: 'f/crm/daily', script_path: 'f/shared/job' })).toBeUndefined()
	})

	it('lets through a write in any folder of a scope of several', () => {
		const several = new WorkingScope(['f/crm', 'f/ops'], alice)
		expect(several.outsidePath({ path: 'f/ops/job' })).toBeUndefined()
		expect(several.outsidePath({ path: 'f/billing/x' })).toBe('f/billing/x')
	})

	it('names every write when no folder is selected, the user space included', () => {
		expect(new WorkingScope([], alice).outsidePath({ path: 'u/alice/x' })).toBe('u/alice/x')
	})

	it('names nothing for the whole workspace', () => {
		expect(new WorkingScope(undefined, alice).outsidePath({ path: 'u/bob/x' })).toBeUndefined()
	})
})

describe('WorkingScope.loadsSkill', () => {
	it("suspends other folders' skills and keeps the user's own space on", () => {
		const scope = new WorkingScope(['f/crm'], alice)
		expect(scope.loadsSkill('f/crm/tone')).toBe(true)
		expect(scope.loadsSkill('u/alice/style')).toBe(true)
		expect(scope.loadsSkill('f/marketing/tone')).toBe(false)
	})
})
