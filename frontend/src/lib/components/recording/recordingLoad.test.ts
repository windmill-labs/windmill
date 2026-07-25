import { describe, expect, it } from 'vitest'
import {
	MAX_ARG_PROPERTIES,
	MAX_FLOW_GROUPS,
	MAX_FLOW_MODULES,
	MAX_FLOW_NOTES,
	MAX_FLOW_STATUS_MODULES,
	MAX_FRAME_STATUSES,
	MAX_RECORDED_JOBS,
	MAX_RENDER_ALL,
	MAX_SAMPLE_CELLS,
	MAX_SAMPLE_COLUMNS,
	parseRecording
} from './recordingLoad'

const job = (events = 1) => ({
	initial_job: { id: 'j' },
	events: Array.from({ length: events }, (_, t) => ({ t, data: { completed: true } }))
})

const header = { recorded_at: '2026-07-25T00:00:00.000Z', total_duration_ms: 1000 }

const pipeline = (extra: Record<string, unknown> = {}) => ({
	version: 1,
	type: 'pipeline',
	folder: 'orders',
	...header,
	graph: { runnables: [], assets: [], edges: [], triggers: [] },
	timeline: [],
	jobs: {},
	...extra
})

const kindOf = (data: unknown) => {
	const res = parseRecording(data)
	return res.ok ? res.loaded.kind : `error: ${res.error}`
}
const rejected = (data: unknown) => parseRecording(data).ok === false

describe('parseRecording', () => {
	it('routes each kind to its player, defaulting a type-less payload to flow', () => {
		expect(kindOf({ version: 1, flow_path: 'f', ...header, jobs: { j: job() } })).toBe('flow')
		expect(
			kindOf({ version: 1, type: 'flow', flow_path: 'f', ...header, jobs: { j: job() } })
		).toBe('flow')
		expect(
			kindOf({
				version: 1,
				type: 'script',
				script_path: 's',
				...header,
				code: 'echo hi',
				language: 'bash',
				job: job()
			})
		).toBe('script')
		expect(kindOf(pipeline())).toBe('pipeline')
		expect(
			kindOf({
				version: 1,
				type: 'app',
				recorded_at: header.recorded_at,
				total_duration_ms: 0,
				viewport: { width: 800, height: 600 },
				frames: ['<html></html>'],
				steps: [{ t: 0, kind: 'click', label: 'Clicked button', before: 0 }]
			})
		).toBe('app')
	})

	it('rejects payloads a player would choke on rather than falling through to flow', () => {
		// A `?src=` origin is arbitrary: a claimed kind must be validated as that
		// kind, never silently reinterpreted.
		expect(rejected({ version: 1, type: 'script', code: 'x', language: 'bash' })).toBe(true)
		expect(rejected({ version: 1, type: 'pipeline', timeline: [], jobs: {} })).toBe(true)
		expect(rejected({ version: 1, type: 'somethingelse', jobs: {} })).toBe(true)
		expect(rejected({ version: 2, jobs: {} })).toBe(true)
		expect(rejected(null)).toBe(true)
		expect(rejected([{ version: 1 }])).toBe(true)
		// Header fields are required by the types and rendered by every player.
		expect(rejected({ version: 1, jobs: { j: job() } })).toBe(true)
		expect(rejected({ version: 1, flow_path: 'f', recorded_at: 'x', jobs: { j: job() } })).toBe(
			true
		)
		// JobLoader replays each event in a timer, where a throw escapes every
		// boundary, so a non-object event has to be caught at load.
		expect(
			rejected({
				version: 1,
				flow_path: 'f',
				...header,
				jobs: { j: { initial_job: {}, events: ['nope'] } }
			})
		).toBe(true)
		// Cardinality, not just shape: each job mounts a JobLoader.
		const tooManyJobs = Object.fromEntries(
			Array.from({ length: MAX_RECORDED_JOBS + 1 }, (_, i) => [`j${i}`, job(0)])
		)
		expect(rejected({ version: 1, flow_path: 'f', ...header, jobs: tooManyJobs })).toBe(true)
	})

	it('bounds the nested structures that drive render cost, not just the job count', () => {
		// One job whose flow_status.modules is huge renders a subtree per entry, so
		// the job/event counts alone would let a small payload freeze the tab.
		const fatModules = Array.from({ length: MAX_FLOW_STATUS_MODULES + 1 }, () => ({
			type: 'Success'
		}))
		expect(
			rejected({
				version: 1,
				flow_path: 'f',
				...header,
				jobs: { j: { initial_job: { flow_status: { modules: fatModules } }, events: [] } }
			})
		).toBe(true)
		// Same via an event carrying the status update rather than the initial job.
		expect(
			rejected({
				version: 1,
				flow_path: 'f',
				...header,
				jobs: {
					j: {
						initial_job: {},
						events: [{ t: 0, data: { flow_status: { modules: fatModules } } }]
					}
				}
			})
		).toBe(true)
		// A pipeline frame's status map is reassigned whole on a timer.
		const fatStatuses = Object.fromEntries(
			Array.from({ length: MAX_FRAME_STATUSES + 1 }, (_, i) => [`p${i}`, { status: 'success' }])
		)
		expect(rejected(pipeline({ timeline: [{ t: 0, statuses: fatStatuses }] }))).toBe(true)
	})

	it('bounds the collections a script and a flow render immediately', () => {
		const props = (n: number) =>
			Object.fromEntries(Array.from({ length: n }, (_, i) => [`a${i}`, { type: 'string' }]))
		const script = (extra: Record<string, unknown>) => ({
			version: 1,
			type: 'script',
			script_path: 's',
			...header,
			code: 'x',
			language: 'bash',
			job: job(),
			...extra
		})
		// SchemaForm / JobArgs render a row per property.
		expect(rejected(script({ args: props(MAX_ARG_PROPERTIES + 1) }))).toBe(true)
		expect(rejected(script({ schema: { properties: props(MAX_ARG_PROPERTIES + 1) } }))).toBe(true)
		expect(kindOf(script({ args: props(3) }))).toBe('script')

		const flowWith = (value: unknown) => ({
			version: 1,
			flow_path: 'f',
			...header,
			jobs: { j: job() },
			flow: { value }
		})
		const modules = (n: number) =>
			Array.from({ length: n }, (_, i) => ({ id: `m${i}`, value: { type: 'identity' } }))
		expect(rejected(flowWith({ modules: modules(MAX_FLOW_MODULES + 1) }))).toBe(true)
		// The count has to be the total across the nested tree, not the top-level
		// array's length — a branch or loop body renders nodes just the same.
		expect(
			rejected(
				flowWith({
					modules: [
						{
							id: 'loop',
							value: { type: 'forloopflow', modules: modules(MAX_FLOW_MODULES + 1) }
						}
					]
				})
			)
		).toBe(true)
		// A branch is a node and an edge even with no modules in it, so counting only
		// branch *contents* would let empty branches ride free.
		expect(
			rejected(
				flowWith({
					modules: [
						{
							id: 'b',
							value: {
								type: 'branchall',
								branches: Array.from({ length: MAX_FLOW_MODULES + 1 }, () => ({ modules: [] }))
							}
						}
					]
				})
			)
		).toBe(true)
		expect(rejected(flowWith({ modules: [], notes: modules(MAX_FLOW_NOTES + 1) }))).toBe(true)
		// Notes and groups are both rendered overlays: capping one only moves the count.
		expect(rejected(flowWith({ modules: [], groups: modules(MAX_FLOW_GROUPS + 1) }))).toBe(true)
		expect(kindOf(flowWith({ modules: modules(3) }))).toBe('flow')
	})

	it('bounds a recorded render_all result, which fans out into nested components', () => {
		const withResult = (result: unknown) => ({
			version: 1,
			flow_path: 'f',
			...header,
			jobs: {
				j: {
					initial_job: { id: 'j' },
					events: [{ t: 0, data: { completed: true, job: { id: 'j', result } } }]
				}
			}
		})
		expect(
			rejected(withResult({ render_all: Array.from({ length: MAX_RENDER_ALL + 1 }, () => 0) }))
		).toBe(true)
		expect(kindOf(withResult({ render_all: [1, 2, 3] }))).toBe('flow')
	})

	it('tells an oversized recording apart from a corrupt one', () => {
		// A wide for-loop flow records a job per iteration, so a real capture can trip
		// this — it must not read as "your recorder wrote a broken file".
		const tooManyJobs = Object.fromEntries(
			Array.from({ length: MAX_RECORDED_JOBS + 1 }, (_, i) => [`j${i}`, job(0)])
		)
		const res = parseRecording({ version: 1, flow_path: 'f', ...header, jobs: tooManyJobs })
		expect(res.ok ? '' : res.error).toMatch(/holds \d+ jobs/)
		const corrupt = parseRecording({ version: 1, flow_path: 'f', ...header, jobs: 'nope' })
		expect(corrupt.ok ? '' : corrupt.error).toBe('Invalid flow recording format.')
	})

	it('bounds an asset sample on its cell product, not just per axis', () => {
		const sample = (rows: number, columns: number) => ({
			assetSamples: {
				'ducklake:main/t': {
					kind: 'ducklake',
					path: 'main/t',
					uri: 'ducklake://main/t',
					rows: Array.from({ length: rows }, () => ({})),
					columns: Array.from({ length: columns }, (_, i) => ({ field: `c${i}` }))
				}
			}
		})
		// Each axis is individually under its cap, so only the product rejects this.
		const overBudget = MAX_SAMPLE_CELLS / MAX_SAMPLE_COLUMNS + 1
		expect(rejected(pipeline(sample(overBudget, MAX_SAMPLE_COLUMNS)))).toBe(true)
		expect(kindOf(pipeline(sample(10, 20)))).toBe('pipeline')
	})
})
