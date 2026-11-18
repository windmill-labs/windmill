import { describe, it, expect } from 'vitest'
import {
	parseDbtRun,
	relationOutcome,
	splitRelation,
	statusRank,
	splitUniqueId,
	nodeSelector
} from './parseDbtRun'

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

	// The failures worth reading are the ones whose message carries braces of its
	// own — a Jinja template, the compiled SQL, an adapter's own JSON — and the
	// payload is appended after all of it.
	it('finds the run past braces in the error text', () => {
		const failed = {
			error: {
				name: 'ExecutionErr',
				message:
					'execution error:\nCompilation Error in model x\n  {{ ref("missing") }} depends on {"a": 1}\n' +
					`Non-zero exit status for dbt build: 1\n\n${JSON.stringify(run)}`
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

	// The payload carries one object per node, so a scan bounded by brace COUNT
	// gives up on an ordinary project — a few hundred nodes, tests included — and
	// silently loses the per-model table on exactly the runs it exists for.
	it('finds the run in a payload with hundreds of nodes', () => {
		const big = {
			...run,
			totals: { total: 400, success: 399, error: 1, warn: 0, skipped: 0 },
			nodes: Array.from({ length: 400 }, (_, i) => ({
				unique_id: `model.p.m${i}`,
				status: i === 0 ? 'error' : 'success'
			}))
		}
		const failed = {
			error: {
				name: 'ExecutionErr',
				message:
					'execution error:\nCompilation Error {{ ref("x") }} {"a": 1}\n\n' +
					JSON.stringify(big, null, 2)
			}
		}
		expect(parseDbtRun(failed)?.nodes?.length).toBe(400)
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

	// The worker publishes `unknown` for a status it does not recognise and counts
	// it in `totals.error`; ranking it as a pass drew a green check on a node the
	// same result called an error.
	// The worker counts `no_op` in totals.skipped — dbt built nothing for that
	// node — so a green check would claim a run that never happened.
	it('ranks a no-op with the skips, not with the passes', () => {
		expect(statusRank('no-op', 'no_op')).toBe(statusRank('skipped', 'skipped'))
		expect(statusRank('success', 'no_op')).toBe(2)
	})

	it('ranks an unknown outcome with the failures, not with the passes', () => {
		expect(statusRank('some-future-dbt-status', 'unknown')).toBe(0)
		expect(statusRank('success', 'unknown')).toBe(0)
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

describe('relationOutcome', () => {
	it('agrees with the worker classifier on every status it names', () => {
		expect(relationOutcome('started')).toBe('running')
		for (const s of ['success', 'pass', 'PASS', ' Success ']) {
			expect(relationOutcome(s)).toBe('materialized')
		}
		// `partial success` built the relation and then failed its tests; the
		// worker records it failed, so the colour must agree.
		for (const s of ['error', 'fail', 'runtime error', 'partial success', 'PARTIAL SUCCESS']) {
			expect(relationOutcome(s)).toBe('failed')
		}
		// Nothing was built, so nothing is coloured.
		for (const s of ['warn', 'skipped', 'no-op', 'something new']) {
			expect(relationOutcome(s)).toBeUndefined()
		}
	})
})

describe('splitRelation', () => {
	it('keeps a period that lives inside a quoted identifier', () => {
		// The backend supports it, so rendering it as `v2.orders` names a
		// relation that does not exist.
		expect(splitRelation('"wh"."analytics.v2"."orders"')).toEqual(['wh', 'analytics.v2', 'orders'])
		expect(splitRelation('"db"."schema"."name"')).toEqual(['db', 'schema', 'name'])
		expect(splitRelation('db.schema.name')).toEqual(['db', 'schema', 'name'])
		// BigQuery backticks and T-SQL brackets quote too.
		expect(splitRelation('`proj`.`data.set`.`t`')).toEqual(['proj', 'data.set', 't'])
		expect(splitRelation('[db].[my.schema].[t]')).toEqual(['db', 'my.schema', 't'])
	})

	// Every one of these dialects escapes its delimiter by doubling it. Dropping
	// the pair renames the relation, and the manifest keeps the real spelling —
	// so the run's status would be recorded against a key no graph node has.
	it('keeps a delimiter the identifier escaped by doubling', () => {
		expect(splitRelation('"wh"."schema"."a""b"')).toEqual(['wh', 'schema', 'a"b'])
		expect(splitRelation('`proj`.`da``ta`.`t`')).toEqual(['proj', 'da`ta', 't'])
		expect(splitRelation('[db].[my]]schema].[t]')).toEqual(['db', 'my]schema', 't'])
	})
})

describe('nodeSelector', () => {
	// Verified against dbt-core 1.12, dbt-core 2.0.0-alpha.5 and fusion
	// 2.0.0-preview.202: the intersection resolves to the one node whatever the
	// project's `model-paths` is, while a path-derived FQN resolves to nothing
	// as soon as that root is more than one segment deep.
	it('intersects the name with its package, wherever the model sits', () => {
		expect(nodeSelector('model.jaffle_shop.fct_orders')).toBe('fct_orders,package:jaffle_shop')
	})

	// Ambiguous across packages, but a selector dbt resolves rather than rejects.
	it('falls back to the bare name without a package', () => {
		expect(nodeSelector('fct_orders')).toBe('fct_orders')
	})
})
