import { describe, expect, it } from 'vitest'
import { runCascade, runSelection } from './cascadeOrchestrator'
import type { DownstreamClosure, InducedSchedule } from './graphTraversal'

function closure(edges: Array<[from: string, to: string]>, nodes: string[]): DownstreamClosure {
	const e = new Map<string, Set<string>>()
	const indegree = new Map<string, number>()
	for (const n of nodes) indegree.set(n, 0)
	for (const [from, to] of edges) {
		const set = e.get(from) ?? new Set()
		set.add(to)
		e.set(from, set)
		indegree.set(to, (indegree.get(to) ?? 0) + 1)
	}
	return { nodes, edges: e, indegree, cyclic: [] }
}

// Deterministic fake backend: launch resolves immediately with `job:<path>`,
// waitTerminal resolves per the `results` table (default success), recording
// the order in which jobs were launched.
function fakeRunner(results: Record<string, 'success' | 'failure'> = {}) {
	const launched: string[] = []
	return {
		launched,
		launch: async (path: string) => {
			launched.push(path)
			return `job:${path}`
		},
		waitTerminal: async (jobId: string) => results[jobId.slice(4)] ?? ('success' as const)
	}
}

describe('runCascade', () => {
	it('runs a linear chain in order', async () => {
		const cl = closure(
			[
				['a', 'b'],
				['b', 'c']
			],
			['b', 'c']
		)
		const r = fakeRunner()
		const res = await runCascade({ closure: cl, root: 'a', ...r })
		expect(r.launched).toEqual(['a', 'b', 'c'])
		expect(res.ok).toBe(true)
		expect(res.statuses.get('c')?.status).toBe('success')
	})

	it('waits for ALL upstreams before running a join node', async () => {
		// a → b, a → c, {b,c} → d
		const cl = closure(
			[
				['a', 'b'],
				['a', 'c'],
				['b', 'd'],
				['c', 'd']
			],
			['b', 'c', 'd']
		)
		const r = fakeRunner()
		await runCascade({ closure: cl, root: 'a', ...r })
		const order = r.launched
		expect(order[0]).toBe('a')
		expect(order.indexOf('d')).toBeGreaterThan(order.indexOf('b'))
		expect(order.indexOf('d')).toBeGreaterThan(order.indexOf('c'))
		expect(order).toHaveLength(4)
	})

	it('fans out independent branches concurrently (both launched before either finishes)', async () => {
		const cl = closure(
			[
				['a', 'b'],
				['a', 'c']
			],
			['b', 'c']
		)
		const launched: string[] = []
		// waitTerminal blocks until BOTH b and c have been launched — only
		// passes if the scheduler launches them without awaiting in between.
		let release!: () => void
		const bothLaunched = new Promise<void>((res) => (release = res))
		const res = await runCascade({
			closure: cl,
			root: 'a',
			launch: async (p) => {
				launched.push(p)
				if (launched.filter((x) => x !== 'a').length === 2) release()
				return `job:${p}`
			},
			waitTerminal: async (jobId) => {
				if (jobId !== 'job:a') await bothLaunched
				return 'success'
			}
		})
		expect(res.ok).toBe(true)
		expect(launched.sort()).toEqual(['a', 'b', 'c'])
	})

	it('stops scheduling after a failure and marks unreached nodes skipped', async () => {
		const cl = closure(
			[
				['a', 'b'],
				['b', 'c']
			],
			['b', 'c']
		)
		const r = fakeRunner({ b: 'failure' })
		const res = await runCascade({ closure: cl, root: 'a', ...r })
		expect(res.ok).toBe(false)
		expect(res.statuses.get('b')?.status).toBe('failure')
		expect(res.statuses.get('c')?.status).toBe('skipped')
		expect(r.launched).toEqual(['a', 'b'])
	})

	it('a launch error is a failure, not an unhandled rejection', async () => {
		const cl = closure([['a', 'b']], ['b'])
		const res = await runCascade({
			closure: cl,
			root: 'a',
			launch: async (p) => {
				if (p === 'b') throw new Error('boom')
				return `job:${p}`
			},
			waitTerminal: async () => 'success'
		})
		expect(res.ok).toBe(false)
		expect(res.statuses.get('b')?.status).toBe('failure')
		expect(res.statuses.get('b')?.error).toContain('boom')
	})

	it('emits monotonic progress snapshots ending in a terminal state for every node', async () => {
		const cl = closure([['a', 'b']], ['b'])
		const snaps: Array<Map<string, { status: string }>> = []
		await runCascade({
			closure: cl,
			root: 'a',
			...fakeRunner(),
			onUpdate: (s) => snaps.push(s)
		})
		const last = snaps[snaps.length - 1]
		expect(last.get('a')?.status).toBe('success')
		expect(last.get('b')?.status).toBe('success')
		// first snapshot: root running, b still pending
		expect(snaps[0].get('a')?.status).toBe('running')
		expect(snaps[0].get('b')?.status).toBe('pending')
	})

	it('root failure skips the whole closure', async () => {
		const cl = closure(
			[
				['a', 'b'],
				['b', 'c']
			],
			['b', 'c']
		)
		const r = fakeRunner({ a: 'failure' })
		const res = await runCascade({ closure: cl, root: 'a', ...r })
		expect(res.ok).toBe(false)
		expect(r.launched).toEqual(['a'])
		expect(res.statuses.get('b')?.status).toBe('skipped')
		expect(res.statuses.get('c')?.status).toBe('skipped')
	})
})

function schedule(
	edges: Array<[from: string, to: string]>,
	nodes: string[],
	roots: string[]
): InducedSchedule {
	const e = new Map<string, Set<string>>()
	const indegree = new Map<string, number>()
	for (const n of nodes) indegree.set(n, 0)
	for (const [from, to] of edges) {
		const set = e.get(from) ?? new Set()
		set.add(to)
		e.set(from, set)
		indegree.set(to, (indegree.get(to) ?? 0) + 1)
	}
	return { nodes, edges: e, indegree, roots, cyclic: [] }
}

describe('runSelection', () => {
	it('runs every selected node, seeding all roots', async () => {
		// two independent roots a, b each → c.
		const sched = schedule(
			[
				['a', 'c'],
				['b', 'c']
			],
			['a', 'b', 'c'],
			['a', 'b']
		)
		const r = fakeRunner()
		const res = await runSelection({ schedule: sched, ...r })
		expect(res.ok).toBe(true)
		// Sort a copy for the membership check — mutating `r.launched` here
		// would invalidate the launch-order assertions below.
		expect([...r.launched].sort()).toEqual(['a', 'b', 'c'])
		// c only after both upstreams.
		expect(r.launched.indexOf('c')).toBeGreaterThan(r.launched.indexOf('a'))
		expect(r.launched.indexOf('c')).toBeGreaterThan(r.launched.indexOf('b'))
	})

	it('skips a failed node’s descendants but keeps independent branches running', async () => {
		// Two independent chains: a → b and c → d. `a` fails; `b` (its descendant)
		// must be skipped, but `d` depends only on the successful `c`, so it must
		// still run — a failure must not stall unrelated branches.
		const sched = schedule(
			[
				['a', 'b'],
				['c', 'd']
			],
			['a', 'b', 'c', 'd'],
			['a', 'c']
		)
		const r = fakeRunner({ a: 'failure' })
		const res = await runSelection({ schedule: sched, ...r })
		expect(res.ok).toBe(false)
		expect(res.statuses.get('a')?.status).toBe('failure')
		expect(res.statuses.get('b')?.status).toBe('skipped')
		expect(res.statuses.get('c')?.status).toBe('success')
		expect(res.statuses.get('d')?.status).toBe('success')
		expect(r.launched).toContain('d')
		expect(r.launched).not.toContain('b')
	})

	it('skips a join node when any one of its upstreams fails', async () => {
		// {a, b} → c. `a` fails; `c` needs both, so it must be skipped even though
		// `b` succeeds — a poisoned lineage isn’t rescued by a sibling success.
		const sched = schedule(
			[
				['a', 'c'],
				['b', 'c']
			],
			['a', 'b', 'c'],
			['a', 'b']
		)
		const r = fakeRunner({ a: 'failure' })
		const res = await runSelection({ schedule: sched, ...r })
		expect(res.ok).toBe(false)
		expect(res.statuses.get('c')?.status).toBe('skipped')
		expect(r.launched).not.toContain('c')
	})

	it('stops scheduling a failed node’s chain', async () => {
		const sched = schedule([['a', 'b']], ['a', 'b'], ['a'])
		const r = fakeRunner({ a: 'failure' })
		const res = await runSelection({ schedule: sched, ...r })
		expect(res.ok).toBe(false)
		expect(r.launched).toEqual(['a'])
		expect(res.statuses.get('b')?.status).toBe('skipped')
	})

	it('a single-node selection runs that node', async () => {
		const sched = schedule([], ['a'], ['a'])
		const r = fakeRunner()
		const res = await runSelection({ schedule: sched, ...r })
		expect(res.ok).toBe(true)
		expect(r.launched).toEqual(['a'])
	})
})
