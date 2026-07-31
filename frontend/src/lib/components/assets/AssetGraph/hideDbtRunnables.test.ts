import { describe, it, expect } from 'vitest'
import { hideDbtRunnables } from './hideDbtRunnables'
import type { AssetGraphResponse } from './types'

// A folder holding both a dbt project (2 models, one of them a shared mart) and
// a pipeline member that reads that mart.
function graph(): AssetGraphResponse {
	return {
		assets: [
			{ kind: 'dbt', path: 'u/a/wh/s/stg_orders', dbt: { unique_id: 'model.p.stg_orders' } },
			{ kind: 'dbt', path: 'u/a/wh/s/fct_orders', dbt: { unique_id: 'model.p.fct_orders' } },
			{ kind: 's3object', path: 'out.csv' }
		] as AssetGraphResponse['assets'],
		runnables: [
			{ path: 'f/x/dbtproj', usage_kind: 'script', dbt: { model_count: 2 } },
			{ path: 'f/x/report', usage_kind: 'script', in_pipeline: true }
		],
		edges: [
			{
				runnable_path: 'f/x/dbtproj',
				runnable_kind: 'script',
				asset_kind: 'dbt',
				asset_path: 'u/a/wh/s/stg_orders',
				access_type: 'w'
			},
			{
				runnable_path: 'f/x/dbtproj',
				runnable_kind: 'script',
				asset_kind: 'dbt',
				asset_path: 'u/a/wh/s/fct_orders',
				access_type: 'w'
			},
			{
				runnable_path: 'f/x/report',
				runnable_kind: 'script',
				asset_kind: 'dbt',
				asset_path: 'u/a/wh/s/fct_orders',
				access_type: 'r'
			}
		],
		triggers: [
			{
				trigger_kind: 'asset',
				asset_kind: 'dbt',
				asset_path: 'u/a/wh/s/fct_orders',
				runnable_kind: 'script',
				runnable_path: 'f/x/report'
			}
		],
		dbt_edges: [{ from_asset_path: 'u/a/wh/s/stg_orders', to_asset_path: 'u/a/wh/s/fct_orders' }]
	}
}

describe('hideDbtRunnables', () => {
	it('drops the dbt script node and its write edges', () => {
		const g = hideDbtRunnables(graph())
		expect(g.runnables.map((r) => r.path)).toEqual(['f/x/report'])
		expect(g.edges.map((e) => e.runnable_path)).toEqual(['f/x/report'])
	})

	it('keeps every model and its ref() lineage — the tables are the pipeline’s business', () => {
		const g = hideDbtRunnables(graph())
		expect(g.assets).toHaveLength(3)
		expect(g.dbt_edges).toHaveLength(1)
		// The cascade edge from the mart to its consumer is what survives.
		expect(g.triggers).toHaveLength(1)
	})

	it('keeps a flow that shares a path with the dbt script', () => {
		// A script and a flow may share a path; the graph keys runnables by
		// `(usage_kind, path)`, so only the dbt script may be removed.
		const base = graph()
		const g = hideDbtRunnables({
			...base,
			runnables: [...base.runnables, { path: 'f/x/dbtproj', usage_kind: 'flow', in_pipeline: true }],
			edges: [
				...base.edges,
				{
					runnable_path: 'f/x/dbtproj',
					runnable_kind: 'flow',
					asset_kind: 's3object',
					asset_path: 'flow-out.csv',
					access_type: 'w'
				}
			]
		})
		expect(g.runnables.map((r) => `${r.usage_kind}:${r.path}`).sort()).toEqual([
			'flow:f/x/dbtproj',
			'script:f/x/report'
		])
		expect(g.edges.some((e) => e.runnable_kind === 'flow')).toBe(true)
	})

	it('is a no-op on a folder with no dbt project', () => {
		const base = graph()
		const plain = { ...base, runnables: base.runnables.filter((r) => !r.dbt) }
		expect(hideDbtRunnables(plain)).toBe(plain)
	})
})
