/**
 * The synthesis is what makes a run-based recording replayable: these pin the
 * contract JobLoader's replay path and FlowStatusViewer depend on — event
 * ordering, log reassembly, module status transitions and timestamp rebasing.
 */
import { describe, expect, it } from 'vitest'
import type { Job } from '$lib/gen'
import { synthesizeFlowReplay, synthesizeSingleJobReplay } from './replayStream'

const T0 = new Date('2026-08-01T10:00:00.000Z').getTime()
const iso = (offsetMs: number) => new Date(T0 + offsetMs).toISOString()

const scriptJob = (extra: Record<string, unknown> = {}): Job =>
	({
		id: 'sub1',
		type: 'CompletedJob',
		success: true,
		created_at: iso(0),
		started_at: iso(100),
		duration_ms: 2000,
		logs: 'line1\nline2\nline3\n',
		result: { ok: true },
		...extra
	}) as any

describe('synthesizeSingleJobReplay', () => {
	it('streams from a queued initial job to the completed one, logs in between', () => {
		const nowMs = 5_000_000
		const stream = synthesizeSingleJobReplay(scriptJob(), { nowMs })

		expect(stream.initial_job.type).toBe('QueuedJob')
		expect((stream.initial_job as any).logs).toBe('')
		expect((stream.initial_job as any).result).toBeUndefined()
		expect((stream.initial_job as any).success).toBeUndefined()

		const events = stream.events
		expect(events[0].data).toEqual({ running: true })
		const completed = events[events.length - 1]
		expect(completed.data.completed).toBe(true)
		expect(completed.t).toBe(2000)
		expect(completed.data.job.result).toEqual({ ok: true })

		// Log chunks sit strictly inside the run window, in order, and reassemble
		// to the original logs; offsets increase so JobLoader appends.
		const logEvents = events.filter((e) => e.data.new_logs)
		expect(logEvents.length).toBeGreaterThan(0)
		for (const e of logEvents) {
			expect(e.t).toBeGreaterThan(0)
			expect(e.t).toBeLessThan(2000)
		}
		expect(logEvents.map((e) => e.data.new_logs).join('')).toBe('line1\nline2\nline3\n')
		const offsets = logEvents.map((e) => e.data.log_offset)
		expect([...offsets].sort((a, b) => a - b)).toEqual(offsets)
		expect(offsets[0]).toBeGreaterThan(0)

		// Absolute timestamps are rebased onto the replay clock ("now").
		expect(new Date(stream.initial_job.started_at!).getTime()).toBe(nowMs)
		expect(new Date(completed.data.job.started_at).getTime()).toBe(nowMs)
	})

	it('collapse reveals the completed job at t=0', () => {
		const stream = synthesizeSingleJobReplay(scriptJob(), { collapse: true, nowMs: 1 })
		expect(stream.events).toHaveLength(1)
		expect(stream.events[0]).toMatchObject({ t: 0, data: { completed: true } })
		expect((stream.initial_job as any).result).toEqual({ ok: true })
	})

	it('survives hostile timestamps without throwing or scheduling into the far future', () => {
		const stream = synthesizeSingleJobReplay(
			scriptJob({ started_at: 'not-a-date', created_at: undefined, duration_ms: 1e18 }),
			{ nowMs: 1 }
		)
		for (const e of stream.events) {
			expect(Number.isFinite(e.t)).toBe(true)
			expect(e.t).toBeLessThanOrEqual(6 * 60 * 60 * 1000)
		}
	})
})

describe('synthesizeFlowReplay', () => {
	const rootJob = (): Job =>
		({
			id: 'root',
			type: 'CompletedJob',
			job_kind: 'flowpreview',
			success: true,
			created_at: iso(0),
			started_at: iso(0),
			duration_ms: 6000,
			flow_status: {
				step: 2,
				modules: [
					{ id: 'a', type: 'Success', job: 'sub1' },
					{ id: 'b', type: 'Success', job: 'sub2' }
				]
			}
		}) as any

	const jobs = (): Record<string, Job> => ({
		root: rootJob(),
		sub1: scriptJob(),
		sub2: scriptJob({
			id: 'sub2',
			started_at: iso(3000),
			duration_ms: 2500,
			logs: 'later\n'
		})
	})

	it('flips module statuses at the sub-jobs’ recorded start/end times', () => {
		const replay = synthesizeFlowReplay(jobs(), 'root', 9_000_000)
		const root = replay.jobs['root']

		// Before anything ran, both modules wait.
		const initialModules = (root.initial_job as any).flow_status.modules
		expect(initialModules.map((m: any) => m.type)).toEqual([
			'WaitingForPriorSteps',
			'WaitingForPriorSteps'
		])
		// A waiting module must not leak its job id — that is what triggers
		// sub-job discovery in the viewer.
		expect(initialModules[0].job).toBeUndefined()

		const statusAt = (t: number) => {
			const snapshots = root.events.filter((e) => e.data.flow_status && e.t <= t)
			return snapshots[snapshots.length - 1]?.data.flow_status.modules.map((m: any) => m.type)
		}
		// sub1 runs 100→2100, sub2 runs 3000→5500.
		expect(statusAt(150)).toEqual(['InProgress', 'WaitingForPriorSteps'])
		expect(statusAt(2200)).toEqual(['Success', 'WaitingForPriorSteps'])
		expect(statusAt(3100)).toEqual(['Success', 'InProgress'])

		// The root completes only after every sub-job event has landed.
		const rootCompleted = root.events.find((e) => e.data.completed)!
		const maxSubT = Math.max(
			...['sub1', 'sub2'].flatMap((id) => replay.jobs[id].events.map((e) => e.t))
		)
		expect(rootCompleted.t).toBeGreaterThan(maxSubT)

		// Sub streams are anchored on the root clock, not their own.
		expect(replay.jobs['sub2'].events[0]).toMatchObject({ t: 3000, data: { running: true } })
	})

	it('never mutates the input jobs, so a second Play replays identically', () => {
		const input = {
			root: {
				...rootJob(),
				flow_status: {
					step: 1,
					modules: [
						{
							id: 'loop',
							type: 'Success',
							flow_jobs: ['it1'],
							flow_jobs_duration: { started_at: [iso(100)], duration_ms: [1500] }
						}
					]
				}
			} as any,
			it1: scriptJob({ id: 'it1', started_at: iso(100), duration_ms: 1500 })
		}
		const before = JSON.stringify(input)
		const first = synthesizeFlowReplay(input, 'root', 9_000_000)
		// The input survives synthesis untouched — snapshots taken at/after a
		// module's end must not alias (and later rebase) its original objects.
		expect(JSON.stringify(input)).toBe(before)

		// A snapshot after the loop ended still shows it completed, with its
		// recorded (now rebased) iteration timing intact.
		const snapshots = first.jobs['root'].events.filter((e) => e.data.flow_status)
		const after = snapshots[snapshots.length - 1].data.flow_status.modules[0]
		expect(after.type).toBe('Success')
		expect(new Date(after.flow_jobs_duration.started_at[0]).getTime()).toBe(9_000_000 + 100)

		const second = synthesizeFlowReplay(input, 'root', 9_000_000)
		expect(JSON.stringify(second)).toBe(JSON.stringify(first))
	})

	it('bounds the aggregate synthesized events across a many-job replay', () => {
		const jobsMap: Record<string, Job> = { root: rootJob() }
		// 200 sub-jobs, each with logs that would individually chunk into 300
		// ticks — unbudgeted that is 60k log events.
		for (let i = 0; i < 200; i++) {
			jobsMap[`s${i}`] = scriptJob({
				id: `s${i}`,
				started_at: iso(100),
				duration_ms: 300_000,
				logs: Array.from({ length: 1000 }, (_, l) => `line ${l}`).join('\n')
			})
		}
		const replay = synthesizeFlowReplay(jobsMap, 'root', 9_000_000)
		const total = Object.values(replay.jobs).reduce((sum, s) => sum + s.events.length, 0)
		// 20k budgeted events plus the per-job running/completed pair and the
		// budget-exempt single log chunk each job is always allowed.
		expect(total).toBeLessThanOrEqual(20_000 + 3 * Object.keys(jobsMap).length)
		// Logs still fully replay even where the budget pinched to one chunk.
		for (const [id, stream] of Object.entries(replay.jobs)) {
			const logs = stream.events
				.filter((e) => e.data.new_logs)
				.map((e) => e.data.new_logs)
				.join('')
			expect(logs).toBe((jobsMap[id] as any).logs ?? '')
		}
	})

	it('keeps a module without recorded timing hidden until the flow completes', () => {
		const jobsMap: Record<string, Job> = {
			root: rootJob(),
			sub1: scriptJob()
			// sub2 (module b's job) deliberately not recorded
		}
		const replay = synthesizeFlowReplay(jobsMap, 'root', 9_000_000)
		const root = replay.jobs['root']
		const states = [
			(root.initial_job as any).flow_status,
			...root.events.filter((e) => e.data.flow_status).map((e) => e.data.flow_status)
		]
		for (const fs of states) {
			// Never reveals its final state — or its job id, which would trigger
			// sub-job discovery for a job the recording doesn't hold.
			expect(fs.modules[1]).toEqual({ id: 'b', type: 'WaitingForPriorSteps' })
		}
		const completed = root.events.find((e) => e.data.completed)!
		expect(completed.data.job.flow_status.modules[1].type).toBe('Success')
	})

	it('completes a nested sub-flow only after its own children', () => {
		// Nested flow and its child share the exact same end instant — the tie the
		// per-stream synthesis cannot break on its own.
		const nested = {
			id: 'nested',
			type: 'CompletedJob',
			job_kind: 'flow',
			parent_job: 'root',
			started_at: iso(100),
			duration_ms: 2000,
			flow_status: { step: 1, modules: [{ id: 'inner', type: 'Success', job: 'leaf' }] }
		} as any
		const leaf = scriptJob({ id: 'leaf', started_at: iso(100), duration_ms: 2000 })
		const outerRoot = {
			...rootJob(),
			flow_status: { step: 1, modules: [{ id: 'n', type: 'Success', job: 'nested' }] }
		} as any
		const replay = synthesizeFlowReplay({ root: outerRoot, nested, leaf }, 'root', 9_000_000)
		const completedT = (id: string) => replay.jobs[id].events.find((e) => e.data.completed)!.t
		const leafMax = replay.jobs['leaf'].events.reduce((m, e) => Math.max(m, e.t), 0)
		expect(completedT('nested')).toBeGreaterThan(leafMax)
		expect(completedT('root')).toBeGreaterThan(completedT('nested'))
	})

	it('survives malformed flow_status shapes without throwing', () => {
		const malformed = scriptJob({
			flow_status: {
				step: 'x',
				modules: {},
				failure_module: { flow_jobs_duration: { started_at: 'nope' } }
			}
		})
		expect(() => synthesizeSingleJobReplay(malformed, { nowMs: 1 })).not.toThrow()
		expect(() => synthesizeSingleJobReplay(malformed, { nowMs: 1, collapse: true })).not.toThrow()
		expect(() => synthesizeFlowReplay({ root: malformed }, 'root', 9_000_000)).not.toThrow()
	})

	it('trims a loop’s iterations to those started at each snapshot', () => {
		const loopRoot = {
			...rootJob(),
			flow_status: {
				step: 1,
				modules: [
					{
						id: 'loop',
						type: 'Success',
						flow_jobs: ['it1', 'it2'],
						flow_jobs_success: [true, true],
						iterator: { index: 1, itered_len: 2 },
						flow_jobs_duration: {
							started_at: [iso(100), iso(2000)],
							duration_ms: [1500, 1500]
						}
					}
				]
			}
		} as any
		const replay = synthesizeFlowReplay(
			{
				root: loopRoot,
				it1: scriptJob({ id: 'it1', started_at: iso(100), duration_ms: 1500 }),
				it2: scriptJob({ id: 'it2', started_at: iso(2000), duration_ms: 1500 })
			},
			'root',
			9_000_000
		)
		const snapshots = replay.jobs['root'].events.filter((e) => e.data.flow_status)
		const during = snapshots.find((e) => e.t >= 100 && e.t < 2000)!
		const mod = during.data.flow_status.modules[0]
		expect(mod.type).toBe('InProgress')
		expect(mod.flow_jobs).toEqual(['it1'])
		expect(mod.iterator.index).toBe(0)
	})
})
