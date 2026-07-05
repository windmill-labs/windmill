import { describe, expect, it } from 'vitest'
import type { AssetGraphResponse, AssetGraphTrigger, NativeTriggerKind } from './types'
import {
	ancestors,
	assetUriToNodeId,
	boundedSet,
	buildLineageDag,
	buildLineageDownstreamMap,
	descendants,
	nonAutorunTriggerScripts,
	reachableCutting,
	scriptNodeId,
	scriptsOf,
	validStarts,
	validFromStarts
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

	it('excludes the start node even on a cycle back to it', () => {
		// a → x → b → y → a (cycle): descendants(a) must not contain a.
		const g = graph({
			writes: [
				['a', 'x'],
				['b', 'y']
			],
			subs: [
				['b', 'x'],
				['a', 'y']
			]
		})
		const dag = buildLineageDag(g)
		expect(descendants(dag, sn('a')).has(sn('a'))).toBe(false)
		expect(ancestors(dag, sn('a')).has(sn('a'))).toBe(false)
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

describe('validFromStarts (mid-DAG selective execution)', () => {
	// a → x → sub (subscriber) → y → reader (pure read). `k` is event-triggered.
	const g = () =>
		graph({
			scripts: ['a', 'sub', 'reader', 'k'],
			writes: [
				['a', 'x'],
				['sub', 'y']
			],
			reads: [['reader', 'y']],
			subs: [['sub', 'x']],
			native: [['kafka', 'k']]
		})

	it('includes mid-DAG subscribers and pure readers, not just roots', () => {
		const from = validFromStarts(g())
		expect(from.has(sn('a'))).toBe(true) // root
		expect(from.has(sn('sub'))).toBe(true) // mid-DAG subscriber — NOT a validStart
		expect(from.has(sn('reader'))).toBe(true) // pure reader
		// The old root-only gate would have rejected the mid-DAG nodes.
		expect(validStarts(g()).has(sn('sub'))).toBe(false)
	})

	it('excludes event-triggered scripts (no run-now gesture)', () => {
		expect(validFromStarts(g()).has(sn('k'))).toBe(false)
	})

	it('keeps a scheduled root that also carries an event trigger', () => {
		// schedule wins over the secondary kafka trigger in validStarts, so the
		// scheduled root stays --from-eligible (regression guard).
		const g2 = graph({
			scripts: ['sched_evt', 'consumer'],
			writes: [['sched_evt', 'x']],
			subs: [['consumer', 'x']],
			native: [
				['schedule', 'sched_evt'],
				['kafka', 'sched_evt']
			]
		})
		expect(validStarts(g2).has(sn('sched_evt'))).toBe(true)
		expect(nonAutorunTriggerScripts(g2).has(sn('sched_evt'))).toBe(true)
		expect(validFromStarts(g2).has(sn('sched_evt'))).toBe(true) // union with roots
	})

	it('runs a mid-DAG start plus its downstream WITHOUT re-running upstream', () => {
		// Starting at `sub`, the unbounded downstream is {sub, y, reader} — `a`/`x`
		// upstream are never pulled in (dbt `--select sub+`).
		const dag = buildLineageDag(g())
		const downstream = new Set([sn('sub'), ...descendants(dag, sn('sub'))])
		expect(scriptsOf(downstream).sort()).toEqual(['reader', 'sub'])
		expect(downstream.has(sn('a'))).toBe(false)
		expect(downstream.has(asset('x'))).toBe(false)
	})
})

describe('reachableCutting (barrier cut for "Run + downstream")', () => {
	// a → x → k(kafka, also reads x) → z → consumer. Starting at `a` and running
	// downstream must cut the event handler `k` AND `consumer` (only reachable
	// through it) — else they'd launch with empty args. Mirrors the CLI cut.
	const g = () =>
		graph({
			scripts: ['a', 'k', 'consumer'],
			writes: [
				['a', 'x'],
				['k', 'z']
			],
			reads: [['k', 'x']],
			subs: [['consumer', 'z']],
			native: [['kafka', 'k']]
		})

	it('detects event-triggered scripts as barriers', () => {
		expect(nonAutorunTriggerScripts(g()).has(sn('k'))).toBe(true)
		expect(nonAutorunTriggerScripts(g()).has(sn('a'))).toBe(false)
	})

	it('cuts an event descendant and its event-only downstream from a run set', () => {
		const dag = buildLineageDag(g())
		const barriers = nonAutorunTriggerScripts(g())
		const runNodes = reachableCutting(dag, [sn('a')], barriers)
		expect(scriptsOf(runNodes).sort()).toEqual(['a']) // k + consumer cut
		expect(runNodes.has(sn('k'))).toBe(false)
		expect(runNodes.has(sn('consumer'))).toBe(false)
	})

	it('protects an explicit start even if it carries an event trigger', () => {
		const dag = buildLineageDag(g())
		// start = k itself (user named it); it runs, and so does its downstream.
		const barriers = new Set([...nonAutorunTriggerScripts(g())].filter((id) => id !== sn('k')))
		const runNodes = reachableCutting(dag, [sn('k')], barriers)
		expect(scriptsOf(runNodes).sort()).toEqual(['consumer', 'k'])
	})

	it('keeps a scheduled event root (and its downstream) reachable from an upstream start', () => {
		// a → x → sched_evt(schedule + kafka) → y → consumer. Running downstream
		// from `a`, sched_evt is a scheduled root so it must NOT be a barrier even
		// though it carries an event trigger — matching the CLI, which excludes all
		// valid roots (`starts`), not just the picked start. Mirror of the page's
		// barrier construction: nonAutorunTriggerScripts minus validStarts minus start.
		const g2 = graph({
			scripts: ['a', 'sched_evt', 'consumer'],
			writes: [
				['a', 'x'],
				['sched_evt', 'y']
			],
			subs: [
				['sched_evt', 'x'],
				['consumer', 'y']
			],
			native: [
				['schedule', 'sched_evt'],
				['kafka', 'sched_evt']
			]
		})
		const dag = buildLineageDag(g2)
		const roots = validStarts(g2)
		const barriers = new Set(
			[...nonAutorunTriggerScripts(g2)].filter((id) => !roots.has(id) && id !== sn('a'))
		)
		const runNodes = reachableCutting(dag, [sn('a')], barriers)
		expect(scriptsOf(runNodes).sort()).toEqual(['a', 'consumer', 'sched_evt'])
		// Without the validStarts exclusion, sched_evt (a kafka handler) would be a
		// barrier and `consumer` would be dropped — the bug this guards.
		const naiveBarriers = new Set([...nonAutorunTriggerScripts(g2)].filter((id) => id !== sn('a')))
		expect(scriptsOf(reachableCutting(dag, [sn('a')], naiveBarriers)).sort()).toEqual(['a'])
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
