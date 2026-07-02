import { describe, expect, it } from 'vitest'
import { runBackfill, type BackfillSliceState } from './backfillRun'

// Deterministic fake backend: launch resolves with `job:<partition>`,
// waitTerminal resolves per the `results` table (default success), recording
// launch order.
function fakeRunner(results: Record<string, 'success' | 'failure'> = {}) {
	const launched: string[] = []
	return {
		launched,
		launch: async (partition: string) => {
			launched.push(partition)
			return `job:${partition}`
		},
		waitTerminal: async (jobId: string) => results[jobId.slice(4)] ?? ('success' as const)
	}
}

describe('runBackfill', () => {
	it('runs slices sequentially in order and reports ok', async () => {
		const r = fakeRunner()
		const res = await runBackfill({
			partitions: ['2026-06-26', '2026-06-27', '2026-06-29'],
			launch: r.launch,
			waitTerminal: r.waitTerminal
		})
		expect(r.launched).toEqual(['2026-06-26', '2026-06-27', '2026-06-29'])
		expect(res.ok).toBe(true)
		expect(res.cancelled).toBe(false)
		expect(res.slices.map((s) => s.status)).toEqual(['success', 'success', 'success'])
		expect(res.slices.map((s) => s.jobId)).toEqual([
			'job:2026-06-26',
			'job:2026-06-27',
			'job:2026-06-29'
		])
	})

	it('continues past a failed slice — each slice is independent', async () => {
		const r = fakeRunner({ '2026-06-27': 'failure' })
		const res = await runBackfill({
			partitions: ['2026-06-26', '2026-06-27', '2026-06-29'],
			launch: r.launch,
			waitTerminal: r.waitTerminal
		})
		expect(r.launched).toEqual(['2026-06-26', '2026-06-27', '2026-06-29'])
		expect(res.ok).toBe(false)
		expect(res.slices.map((s) => s.status)).toEqual(['success', 'failure', 'success'])
	})

	it('records a launch error as slice failure and keeps going', async () => {
		const r = fakeRunner()
		const res = await runBackfill({
			partitions: ['a', 'b'],
			launch: async (p) => {
				if (p === 'a') throw new Error('boom')
				return r.launch(p)
			},
			waitTerminal: r.waitTerminal
		})
		expect(res.slices[0]).toMatchObject({ status: 'failure', error: 'boom' })
		expect(res.slices[1].status).toBe('success')
	})

	it('stops before the next launch when cancelled, leaving the rest pending', async () => {
		const r = fakeRunner()
		let done = 0
		const res = await runBackfill({
			partitions: ['a', 'b', 'c'],
			launch: r.launch,
			waitTerminal: async (id) => {
				done++
				return r.waitTerminal(id)
			},
			isCancelled: () => done >= 1
		})
		expect(r.launched).toEqual(['a'])
		expect(res.cancelled).toBe(true)
		expect(res.ok).toBe(false)
		expect(res.slices.map((s) => s.status)).toEqual(['success', 'pending', 'pending'])
	})

	it('emits a snapshot per transition, never mutating earlier snapshots', async () => {
		const r = fakeRunner()
		const snapshots: BackfillSliceState[][] = []
		await runBackfill({
			partitions: ['a'],
			launch: r.launch,
			waitTerminal: r.waitTerminal,
			onUpdate: (s) => snapshots.push(s)
		})
		// initial pending, running, running+jobId, terminal
		expect(snapshots.map((s) => s[0].status)).toEqual(['pending', 'running', 'running', 'success'])
		expect(snapshots[1][0].jobId).toBeUndefined()
		expect(snapshots[2][0].jobId).toBe('job:a')
	})
})
