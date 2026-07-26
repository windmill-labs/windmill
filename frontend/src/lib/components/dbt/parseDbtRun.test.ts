import { describe, it, expect } from 'vitest'
import { parseDbtRun, statusRank, splitUniqueId } from './parseDbtRun'

const run = {
	engine: 'dbt-core-1x',
	engine_version: '1.12.0',
	command: 'build',
	totals: { total: 1, success: 1, error: 0, warn: 0, skipped: 0 },
	nodes: [{ unique_id: 'model.p.customers', status: 'success' }]
}

describe('parseDbtRun', () => {
	it('takes a successful run as-is', () => {
		expect(parseDbtRun(run)?.engine).toBe('dbt-core-1x')
	})

	// The worker puts the same JSON in the error message after the exit-status
	// line, and this is the case worth rendering: the failing node is what the
	// user came for.
	it('recovers the run from a failed job’s error message', () => {
		const failed = {
			error: {
				name: 'ExecutionErr',
				message: `execution error:\nNon-zero exit status for dbt build: 1\n\n${JSON.stringify(run)}`
			}
		}
		expect(parseDbtRun(failed)?.totals?.total).toBe(1)
	})

	// `{nodes, totals}` alone is a shape an ordinary script can return, and it
	// would then be rendered as somebody's dbt run.
	it('does not claim an ordinary result that happens to have nodes and totals', () => {
		expect(parseDbtRun({ nodes: [], totals: {} })).toBeUndefined()
		expect(parseDbtRun({ engine: 'v8', nodes: [], totals: {} })).toBeUndefined()
	})

	it('accepts every engine the worker stamps', () => {
		for (const engine of ['dbt-core-1x', 'dbt-core-2x', 'fusion']) {
			expect(parseDbtRun({ ...run, engine })?.engine).toBe(engine)
		}
	})

	it('is undefined for anything unparseable', () => {
		expect(parseDbtRun(undefined)).toBeUndefined()
		expect(parseDbtRun('a string')).toBeUndefined()
		expect(parseDbtRun({ error: { message: 'failed with {not json' } })).toBeUndefined()
	})
})

describe('statusRank', () => {
	// dbt counts `partial success` in totals.error and a retry redoes it, so
	// ranking it as a pass would contradict the job's own outcome.
	it('ranks partial success with the failures', () => {
		expect(statusRank('partial success')).toBe(statusRank('error'))
		expect(statusRank('PARTIAL SUCCESS')).toBe(0)
	})

	it('orders failed before warned before skipped before passed', () => {
		expect(
			['success', 'skipped', 'warn', 'error'].sort((a, b) => statusRank(a) - statusRank(b))
		).toEqual(['error', 'warn', 'skipped', 'success'])
	})
})

describe('splitUniqueId', () => {
	it('splits kind from name and drops a generic test’s uniqueness hash', () => {
		expect(splitUniqueId('model.jaffle.stg_orders')).toEqual({
			kind: 'model',
			name: 'stg_orders'
		})
		expect(splitUniqueId('test.jaffle.not_null_orders_id.4e687af8d0')).toEqual({
			kind: 'test',
			name: 'not_null_orders_id'
		})
		// A model whose name contains a dot keeps it: only tests carry the hash.
		expect(splitUniqueId('model.jaffle.a.b').name).toBe('a.b')
	})
})
