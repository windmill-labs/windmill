import { describe, expect, it } from 'vitest'
import type { AssetGraphResponse, AssetGraphTrigger, NativeTriggerKind } from './types'
import {
	ancestors,
	assetUriToNodeId,
	boundedSet,
	buildLineageDag,
	buildLineageDownstreamMap,
	descendants,
	scriptNodeId,
	scriptsOf,
	validStarts
} from './boundedCascade'
import { computeInducedSchedule } from './graphTraversal'

type W = [script: string, asset: string] // producer write edge (datatable)
type R = [script: string, asset: string] // pure-read edge (datatable)
type S = [script: string, asset: string] // `// on <asset>` subscription

function graph(opts: {
	scripts?: string[]
	writes?: W[]
	reads?: R[]
	subs?: S[]
	native?: Array<[kind: NativeTriggerKind, script: string]>
}): AssetGraphResponse {
	const { scripts = [], writes = [], reads = [], subs = [], native = [] } = opts
	const triggers: AssetGraphTrigger[] = [
		...subs.map(
			([s, a]) =>
				({
					trigger_kind: 'asset',
					asset_kind: 'datatable',
					asset_path: a,
					runnable_kind: 'script',
					runnable_path: s
				}) as const
		),
		...native.map(
			([kind, s]) =>
				({ trigger_kind: kind, runnable_kind: 'script', runnable_path: s }) as AssetGraphTrigger
		)
	]
	return {
		assets: [],
		runnables: scripts.map((p) => ({ path: p, usage_kind: 'script' as const })),
		edges: [
			...writes.map(([s, a]) => ({
				runnable_path: s,
				runnable_kind: 'script' as const,
				asset_kind: 'datatable' as const,
				asset_path: a,
				access_type: 'w' as const
			})),
			...reads.map(([s, a]) => ({
				runnable_path: s,
				runnable_kind: 'script' as const,
				asset_kind: 'datatable' as const,
				asset_path: a,
				access_type: 'r' as const
			}))
		],
		triggers
	}
}

const sn = scriptNodeId
const asset = (p: string) => `datatable:${p}`

describe('buildLineageDag', () => {
	it('links producer → asset → subscriber and asset → reader', () => {
		// a writes x; b subscribes to x; c reads x.
		const g = graph({ writes: [['a', 'x']], subs: [['b', 'x']], reads: [['c', 'x']] })
		const dag = buildLineageDag(g)
		expect([...(dag.down.get(sn('a')) ?? [])]).toEqual([asset('x')])
		expect([...(dag.down.get(asset('x')) ?? [])].sort()).toEqual([sn('b'), sn('c')])
	})

	it('treats rw as production only (no self-cycle through the asset)', () => {
		const g: AssetGraphResponse = {
			assets: [],
			runnables: [{ path: 'u', usage_kind: 'script' }],
			edges: [
				{
					runnable_path: 'u',
					runnable_kind: 'script',
					asset_kind: 'datatable',
					asset_path: 'x',
					access_type: 'rw'
				}
			],
			triggers: []
		}
		const dag = buildLineageDag(g)
		expect([...(dag.down.get(sn('u')) ?? [])]).toEqual([asset('x')])
		expect(dag.up.get(sn('u'))).toBeUndefined() // asset is not upstream of its own writer
	})
})

describe('ancestors / descendants', () => {
	it('walks transitively over scripts and assets', () => {
		// a → x → b → y → c
		const g = graph({
			writes: [
				['a', 'x'],
				['b', 'y']
			],
			subs: [
				['b', 'x'],
				['c', 'y']
			]
		})
		const dag = buildLineageDag(g)
		expect(descendants(dag, sn('a'))).toEqual(new Set([asset('x'), sn('b'), asset('y'), sn('c')]))
		expect(ancestors(dag, sn('c'))).toEqual(new Set([asset('y'), sn('b'), asset('x'), sn('a')]))
	})
})

describe('boundedSet', () => {
	// a → x → b → y → c → z → d  (linear chain through assets)
	const chain = () =>
		graph({
			writes: [
				['a', 'x'],
				['b', 'y'],
				['c', 'z']
			],
			subs: [
				['b', 'x'],
				['c', 'y'],
				['d', 'z']
			]
		})

	it('stops at a single end node (script)', () => {
		const dag = buildLineageDag(chain())
		const res = boundedSet(dag, sn('a'), [sn('c')])
		// path a..c includes a,x,b,y,c — not z or d.
		expect(scriptsOf(res.nodes).sort()).toEqual(['a', 'b', 'c'])
		expect(res.nodes.has(asset('z'))).toBe(false)
		expect(res.droppedEnds).toEqual([])
	})

	it('supports an asset as the end bound', () => {
		const dag = buildLineageDag(chain())
		const res = boundedSet(dag, sn('a'), [asset('y')])
		// up to datatable://y → a, b produced it.
		expect(scriptsOf(res.nodes).sort()).toEqual(['a', 'b'])
	})

	it('unions multiple ends', () => {
		// diamond: a → b and a → c, both → d. Bound to {b, c} excludes d.
		const g = graph({
			writes: [
				['a', 'xa'],
				['b', 'xb'],
				['c', 'xc']
			],
			subs: [
				['b', 'xa'],
				['c', 'xa'],
				['d', 'xb'],
				['d', 'xc']
			]
		})
		const dag = buildLineageDag(g)
		const res = boundedSet(dag, sn('a'), [sn('b'), sn('c')])
		expect(scriptsOf(res.nodes).sort()).toEqual(['a', 'b', 'c'])
	})

	it('drops ends not downstream of start', () => {
		const dag = buildLineageDag(chain())
		const res = boundedSet(dag, sn('c'), [sn('a')])
		expect(res.droppedEnds).toEqual([sn('a')])
		expect(res.reachableEnds).toEqual([])
		expect([...res.nodes]).toEqual([sn('c')])
	})

	it('is cycle-safe', () => {
		// b ↔ c cycle downstream of a; bounding to c terminates.
		const g = graph({
			writes: [
				['a', 'x'],
				['b', 'y'],
				['c', 'z']
			],
			subs: [
				['b', 'x'],
				['c', 'y'],
				['b', 'z']
			]
		})
		const dag = buildLineageDag(g)
		const res = boundedSet(dag, sn('a'), [sn('c')])
		expect(scriptsOf(res.nodes).sort()).toEqual(['a', 'b', 'c'])
	})
})

describe('validStarts', () => {
	it('includes schedule-rooted scripts', () => {
		const g = graph({ scripts: ['s'], native: [['schedule', 's']] })
		expect(validStarts(g)).toEqual(new Set([sn('s')]))
	})

	it('includes manual roots (no trigger, not a subscriber)', () => {
		const g = graph({ scripts: ['m'] })
		expect(validStarts(g)).toEqual(new Set([sn('m')]))
	})

	it('excludes event-only roots', () => {
		const g = graph({ scripts: ['k'], native: [['kafka', 'k']] })
		expect(validStarts(g).has(sn('k'))).toBe(false)
	})

	it('excludes pure asset subscribers but keeps a schedule subscriber', () => {
		// sub is `// on x`; sched is both a subscriber and schedule-triggered.
		const g = graph({
			scripts: ['a', 'sub', 'sched'],
			writes: [['a', 'x']],
			subs: [
				['sub', 'x'],
				['sched', 'x']
			],
			native: [['schedule', 'sched']]
		})
		const starts = validStarts(g)
		expect(starts.has(sn('a'))).toBe(true) // manual root
		expect(starts.has(sn('sub'))).toBe(false) // event-less but a subscriber
		expect(starts.has(sn('sched'))).toBe(true) // schedule overrides subscriber
	})
})

describe('buildLineageDownstreamMap (read-aware scheduling)', () => {
	// a writes x; c only *reads* x (no `// on x`). c must still run after a.
	const g = graph({
		scripts: ['a', 'c'],
		writes: [['a', 'x']],
		reads: [['c', 'x']]
	})

	it('links a producer to a pure reader of its asset', () => {
		const map = buildLineageDownstreamMap(g)
		expect([...(map.get('a') ?? [])]).toEqual(['c'])
	})

	it('orders the reader after the producer through computeInducedSchedule', () => {
		const selected = new Set(['a', 'c'])
		// Subscriber-only map (default) misses the read dep → c is a stray root.
		const subscriberOnly = computeInducedSchedule(g, selected)
		expect(subscriberOnly.roots.sort()).toEqual(['a', 'c'])
		// Read-aware map orders c strictly after a.
		const readAware = computeInducedSchedule(g, selected, buildLineageDownstreamMap(g))
		expect(readAware.roots).toEqual(['a'])
		expect(readAware.indegree.get('c')).toBe(1)
		expect(readAware.nodes).toEqual(['a', 'c'])
	})
})

describe('assetUriToNodeId', () => {
	it('maps s3 prefix to the s3object kind, others verbatim', () => {
		expect(assetUriToNodeId('s3://bucket/key')).toBe('s3object:bucket/key')
		expect(assetUriToNodeId('datatable://prod/users')).toBe('datatable:prod/users')
		expect(assetUriToNodeId('ducklake://lake/t')).toBe('ducklake:lake/t')
		expect(assetUriToNodeId('not-a-uri')).toBeUndefined()
	})
})
