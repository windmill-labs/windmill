import { describe, expect, it } from 'vitest'
import type { AssetGraphResponse } from './types'
import {
	buildDownstreamMap,
	computeDownstreamClosure,
	computeInducedSchedule
} from './graphTraversal'

/** Direct (one-hop) subscriber script paths of `scriptPath`. */
function downstreamSubscribers(g: AssetGraphResponse, scriptPath: string): string[] {
	return Array.from(buildDownstreamMap(g).get(scriptPath) ?? [])
}

// Minimal graph builder: `writes` are producer→asset write edges, `subs` are
// `// on <asset>` subscriptions. All datatable/script for brevity.
function graph(
	writes: Array<[script: string, asset: string]>,
	subs: Array<[script: string, asset: string]>
): AssetGraphResponse {
	return {
		assets: [],
		runnables: [],
		edges: writes.map(([s, a]) => ({
			runnable_path: s,
			runnable_kind: 'script' as const,
			asset_kind: 'datatable' as const,
			asset_path: a,
			access_type: 'w' as const
		})),
		triggers: subs.map(([s, a]) => ({
			trigger_kind: 'asset' as const,
			asset_kind: 'datatable' as const,
			asset_path: a,
			runnable_kind: 'script' as const,
			runnable_path: s
		}))
	}
}

describe('buildDownstreamMap', () => {
	it('links writers to subscribers through the asset', () => {
		const g = graph(
			[['a', 'x']],
			[
				['b', 'x'],
				['c', 'x']
			]
		)
		expect(downstreamSubscribers(g, 'a').sort()).toEqual(['b', 'c'])
	})

	it('ignores read edges, flow subscribers and self-loops', () => {
		const g = graph([['a', 'x']], [['a', 'x']])
		// reader of x, not writer
		g.edges.push({
			runnable_path: 'r',
			runnable_kind: 'script',
			asset_kind: 'datatable',
			asset_path: 'x',
			access_type: 'r'
		})
		g.triggers.push({
			trigger_kind: 'asset',
			asset_kind: 'datatable',
			asset_path: 'x',
			runnable_kind: 'flow',
			runnable_path: 'f'
		})
		expect(downstreamSubscribers(g, 'a')).toEqual([])
		expect(downstreamSubscribers(g, 'r')).toEqual([])
	})

	it('matches on asset kind as well as path', () => {
		const g = graph([['a', 'x']], [])
		g.triggers.push({
			trigger_kind: 'asset',
			asset_kind: 's3object',
			asset_path: 'x',
			runnable_kind: 'script',
			runnable_path: 'b'
		})
		expect(downstreamSubscribers(g, 'a')).toEqual([])
	})
})

describe('computeDownstreamClosure', () => {
	it('returns the transitive closure in topological order', () => {
		// a → b → c, a → c (diamond-ish): c must come after b.
		const g = graph(
			[
				['a', 'ab'],
				['a', 'ac'],
				['b', 'bc']
			],
			[
				['b', 'ab'],
				['c', 'ac'],
				['c', 'bc']
			]
		)
		const cl = computeDownstreamClosure(g, 'a')
		expect(cl.nodes).toEqual(['b', 'c'])
		expect(cl.cyclic).toEqual([])
		// c has two in-closure upstreams: a (root) and b.
		expect(cl.indegree.get('c')).toBe(2)
		expect(cl.indegree.get('b')).toBe(1)
		expect([...(cl.edges.get('a') ?? [])].sort()).toEqual(['b', 'c'])
		expect([...(cl.edges.get('b') ?? [])]).toEqual(['c'])
	})

	it('multi-hop chains include the full transitive set', () => {
		const g = graph(
			[
				['a', 'x1'],
				['b', 'x2'],
				['c', 'x3']
			],
			[
				['b', 'x1'],
				['c', 'x2'],
				['d', 'x3']
			]
		)
		const cl = computeDownstreamClosure(g, 'a')
		expect(cl.nodes).toEqual(['b', 'c', 'd'])
		expect(cl.indegree.get('b')).toBe(1)
		expect(cl.indegree.get('d')).toBe(1)
	})

	it('excludes the root from nodes even when it is re-triggered (cycle through root)', () => {
		// a → b → a: b is reachable and schedulable; the back-edge to root is dropped.
		const g = graph(
			[
				['a', 'x'],
				['b', 'y']
			],
			[
				['b', 'x'],
				['a', 'y']
			]
		)
		const cl = computeDownstreamClosure(g, 'a')
		expect(cl.nodes).toEqual(['b'])
		expect(cl.cyclic).toEqual([])
	})

	it('reports nodes on a downstream cycle as cyclic instead of hanging', () => {
		// a → b, then b ↔ c cycle, and d hangs off c (schedulable only through
		// the cycle → also cyclic).
		const g = graph(
			[
				['a', 'x'],
				['b', 'y'],
				['c', 'z'],
				['c', 'w']
			],
			[
				['b', 'x'],
				['c', 'y'],
				['b', 'z'],
				['d', 'w']
			]
		)
		const cl = computeDownstreamClosure(g, 'a')
		// b is fed by both root and the cycle — its cycle-edge keeps it un-runnable.
		expect(cl.nodes).toEqual([])
		expect(cl.cyclic.sort()).toEqual(['b', 'c', 'd'])
	})

	it('empty closure for a leaf', () => {
		const g = graph([['a', 'x']], [['b', 'x']])
		const cl = computeDownstreamClosure(g, 'b')
		expect(cl.nodes).toEqual([])
		expect(cl.cyclic).toEqual([])
	})
})

describe('computeInducedSchedule', () => {
	// a → b → c → d linear chain through assets.
	const chain = () =>
		graph(
			[
				['a', 'xa'],
				['b', 'xb'],
				['c', 'xc']
			],
			[
				['b', 'xa'],
				['c', 'xb'],
				['d', 'xc']
			]
		)

	it('orders a subset topologically and keeps only in-set edges', () => {
		const sched = computeInducedSchedule(chain(), new Set(['a', 'b', 'c']))
		expect(sched.nodes).toEqual(['a', 'b', 'c'])
		expect(sched.roots).toEqual(['a'])
		expect(sched.indegree.get('b')).toBe(1)
		expect([...(sched.edges.get('c') ?? [])]).toEqual([]) // d is out of set
		expect(sched.cyclic).toEqual([])
	})

	it('a gap in the selected set produces two independent roots', () => {
		// pick a and c (skip b): with b excluded there is no in-set edge, so both
		// are roots.
		const sched = computeInducedSchedule(chain(), new Set(['a', 'c']))
		expect(sched.roots.sort()).toEqual(['a', 'c'])
		expect(sched.indegree.get('c')).toBe(0)
	})

	it('reports cyclic members instead of hanging', () => {
		// b ↔ c cycle.
		const g = graph(
			[
				['b', 'y'],
				['c', 'z']
			],
			[
				['c', 'y'],
				['b', 'z']
			]
		)
		const sched = computeInducedSchedule(g, new Set(['b', 'c']))
		expect(sched.nodes).toEqual([])
		expect(sched.cyclic.sort()).toEqual(['b', 'c'])
	})
})
