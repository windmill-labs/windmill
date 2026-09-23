import { describe, expect, it } from 'vitest'
import type { AssetGraphResponse } from './types'
import {
	buildColumnGraph,
	buildDbtColumnGraph,
	colNodeId,
	mergeColumnGraphs,
	type ColumnLineageGraph,
	traceColumn,
	connectedComponent,
	assetColumnNodes,
	computeDepths
} from './columnLineageGraph'

// Two scripts chained through an intermediate ducklake table:
//   s1: orders.amount      -> staging.amt
//   s2: staging.amt        -> daily.total
//   s2: customers.name     -> daily.cust   (a second source into the sink)
function chainGraph(): AssetGraphResponse {
	return {
		assets: [],
		triggers: [],
		runnables: [
			{
				path: 's1',
				usage_kind: 'script',
				column_lineage: [
					{
						column: 'amt',
						inputs: [{ from_kind: 'ducklake', from_path: 'wh/orders', from_column: 'amount' }]
					}
				]
			},
			{
				path: 's2',
				usage_kind: 'script',
				column_lineage: [
					{
						column: 'total',
						inputs: [{ from_kind: 'ducklake', from_path: 'wh/staging', from_column: 'amt' }]
					},
					{
						column: 'cust',
						inputs: [{ from_kind: 'ducklake', from_path: 'wh/customers', from_column: 'name' }]
					}
				]
			}
		],
		edges: [
			{
				runnable_path: 's1',
				runnable_kind: 'script',
				asset_kind: 'ducklake',
				asset_path: 'wh/staging',
				access_type: 'w'
			},
			{
				runnable_path: 's2',
				runnable_kind: 'script',
				asset_kind: 'ducklake',
				asset_path: 'wh/daily',
				access_type: 'w'
			}
		]
	}
}

const ORDERS_AMOUNT = colNodeId('ducklake', 'wh/orders', 'amount')
const STAGING_AMT = colNodeId('ducklake', 'wh/staging', 'amt')
const DAILY_TOTAL = colNodeId('ducklake', 'wh/daily', 'total')
const DAILY_CUST = colNodeId('ducklake', 'wh/daily', 'cust')
const CUSTOMERS_NAME = colNodeId('ducklake', 'wh/customers', 'name')

describe('buildColumnGraph', () => {
	it('stitches per-script lineage into a transitive graph via shared columns', () => {
		const g = buildColumnGraph(chainGraph())
		// orders.amount feeds staging.amt feeds daily.total
		expect(g.up.get(STAGING_AMT)).toEqual(new Set([ORDERS_AMOUNT]))
		expect(g.up.get(DAILY_TOTAL)).toEqual(new Set([STAGING_AMT]))
		expect(g.down.get(ORDERS_AMOUNT)).toEqual(new Set([STAGING_AMT]))
		expect(g.down.get(STAGING_AMT)).toEqual(new Set([DAILY_TOTAL]))
	})

	it('anchors to the // materialize target, not a guessed write-edge', () => {
		const graph = chainGraph()
		// s1 declares its materialize target and also has unordered extra
		// ducklake writes; the lineage must anchor to the declared target.
		const s1 = graph.runnables.find((r) => r.path === 's1')!
		s1.materialize_target = { kind: 'ducklake', path: 'wh/staging' }
		graph.edges.unshift({
			runnable_path: 's1',
			runnable_kind: 'script',
			asset_kind: 'ducklake',
			asset_path: 'wh/other',
			access_type: 'w'
		})
		const g = buildColumnGraph(graph)
		expect(g.nodes.has(STAGING_AMT)).toBe(true) // anchored to the declared target
		expect(g.nodes.has(colNodeId('ducklake', 'wh/other', 'amt'))).toBe(false)
	})

	it('falls back to a ducklake write-edge when there is no materialize target', () => {
		// chainGraph's runnables carry no materialize_target, so s1's lineage is
		// anchored via its (single) ducklake write-edge.
		const g = buildColumnGraph(chainGraph())
		expect(g.nodes.has(STAGING_AMT)).toBe(true)
	})

	it('node ids are collision-proof across `#` / `:` in paths and columns', () => {
		// A delimiter-concatenated id would merge these; the JSON-encoded id must not.
		expect(colNodeId('ducklake', 'a#b', 'c')).not.toBe(colNodeId('ducklake', 'a', 'b#c'))
		expect(colNodeId('ducklake', 'a:b', 'c')).not.toBe(colNodeId('ducklake', 'a', 'b:c'))
	})

	it('skips producers with no ducklake output asset (columns unanchorable)', () => {
		const graph = chainGraph()
		graph.edges = graph.edges.filter((e) => e.runnable_path !== 's1') // s1 loses its output edge
		const g = buildColumnGraph(graph)
		// staging.amt is no longer produced as a node by s1...
		expect(g.up.has(STAGING_AMT)).toBe(false)
		// ...but s2 still anchors daily.total ← staging.amt (staging.amt as a source).
		expect(g.up.get(DAILY_TOTAL)).toEqual(new Set([STAGING_AMT]))
	})
})

describe('buildDbtColumnGraph', () => {
	it('takes the direct kinds and drops any other', () => {
		// `scan` means the column was read to produce the ROW — a join key, a
		// predicate, a `group by` — so it reaches every output column of its model
		// and is not what a column trace means. The server filters it out; this
		// filters again, because the kind set is the engine's and an unknown one
		// must not become an edge the trace calls data flow.
		const g = buildDbtColumnGraph([
			{
				from_asset_path: 'main/s/stg',
				from_column: 'raw_name',
				to_asset_path: 'main/s/mart',
				to_column: 'clean_name',
				kind: 'mod'
			},
			{
				from_asset_path: 'main/s/stg',
				from_column: 'id',
				to_asset_path: 'main/s/mart',
				to_column: 'id',
				kind: 'copy'
			},
			{
				from_asset_path: 'main/s/stg',
				from_column: 'id',
				to_asset_path: 'main/s/mart',
				to_column: 'clean_name',
				kind: 'scan'
			}
		])
		expect(g.up.get(colNodeId('dbt', 'main/s/mart', 'clean_name'))).toEqual(
			new Set([colNodeId('dbt', 'main/s/stg', 'raw_name')])
		)
		expect(g.up.get(colNodeId('dbt', 'main/s/mart', 'id'))).toEqual(
			new Set([colNodeId('dbt', 'main/s/stg', 'id')])
		)
	})
})

describe('mergeColumnGraphs', () => {
	it('chains a dbt column into what a producer derives from it', () => {
		// The two halves arrive separately — the producer's from the asset graph,
		// dbt's from its own request — and meet at the dbt node a `// column`
		// annotation names. A trace has to cross that, or a dbt selection stops
		// before the script consuming it.
		const dbt = buildDbtColumnGraph([
			{
				from_asset_path: 'main/s/stg',
				from_column: 'raw',
				to_asset_path: 'main/s/mart',
				to_column: 'clean',
				kind: 'copy'
			}
		])
		const producer: ColumnLineageGraph = {
			nodes: new Map(),
			up: new Map(),
			down: new Map()
		}
		const src = colNodeId('dbt', 'main/s/mart', 'clean')
		const out = colNodeId('ducklake', 'wh/report', 'total')
		producer.nodes.set(src, { kind: 'dbt', path: 'main/s/mart', column: 'clean' })
		producer.nodes.set(out, { kind: 'ducklake', path: 'wh/report', column: 'total' })
		producer.up.set(out, new Set([src]))
		producer.down.set(src, new Set([out]))

		const merged = mergeColumnGraphs(dbt, producer)
		expect(traceColumn(colNodeId('dbt', 'main/s/stg', 'raw'), merged)).toEqual(
			new Set([colNodeId('dbt', 'main/s/stg', 'raw'), src, out])
		)
	})
})

describe('traceColumn', () => {
	it('returns the full upstream + downstream impact set of a source column', () => {
		const g = buildColumnGraph(chainGraph())
		// from the root source, the whole chain downstream is impacted
		expect(traceColumn(ORDERS_AMOUNT, g)).toEqual(
			new Set([ORDERS_AMOUNT, STAGING_AMT, DAILY_TOTAL])
		)
	})

	it('traces backward from a sink to every contributing source', () => {
		const g = buildColumnGraph(chainGraph())
		expect(traceColumn(DAILY_TOTAL, g)).toEqual(new Set([DAILY_TOTAL, STAGING_AMT, ORDERS_AMOUNT]))
		// the sibling output `cust` and its source are NOT in total's trace
		expect(traceColumn(DAILY_TOTAL, g).has(DAILY_CUST)).toBe(false)
		expect(traceColumn(DAILY_TOTAL, g).has(CUSTOMERS_NAME)).toBe(false)
	})

	it('traces an intermediate column both directions', () => {
		const g = buildColumnGraph(chainGraph())
		expect(traceColumn(STAGING_AMT, g)).toEqual(new Set([STAGING_AMT, ORDERS_AMOUNT, DAILY_TOTAL]))
	})
})

describe('connectedComponent + depths', () => {
	it('collects the neighborhood of an asset and lays it out by hop depth', () => {
		const g = buildColumnGraph(chainGraph())
		const seeds = assetColumnNodes(g, 'ducklake', 'wh/daily') // the sink asset
		expect(new Set(seeds)).toEqual(new Set([DAILY_TOTAL, DAILY_CUST]))
		const comp = connectedComponent(seeds, g)
		expect(comp).toEqual(
			new Set([DAILY_TOTAL, DAILY_CUST, STAGING_AMT, ORDERS_AMOUNT, CUSTOMERS_NAME])
		)
		const depths = computeDepths(comp, g)
		expect(depths.get(ORDERS_AMOUNT)).toBe(0)
		expect(depths.get(STAGING_AMT)).toBe(1)
		expect(depths.get(DAILY_TOTAL)).toBe(2)
		expect(depths.get(CUSTOMERS_NAME)).toBe(0)
		expect(depths.get(DAILY_CUST)).toBe(1)
	})
})
