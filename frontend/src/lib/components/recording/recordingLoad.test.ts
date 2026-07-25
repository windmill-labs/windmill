import { describe, expect, it } from 'vitest'
import {
	MAX_FLOW_MODULES,
	MAX_RECORDED_JOBS,
	MAX_SAMPLE_CELLS,
	MAX_SAMPLE_COLUMNS,
	MAX_VALUE_DEPTH,
	MAX_VALUE_NODES,
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

	it('holds every recorded value to one structural render budget', () => {
		// Each of these was previously a named per-key cap, and each of those caps was
		// found to have an unbounded sibling or an unbounded recursion. They are one
		// test now because they are one rule: a value a component expands eagerly may
		// not expand into more than MAX_VALUE_NODES of structure, at any depth.
		const wide = (n: number) => Array.from({ length: n }, () => 0)
		const overBudget = MAX_VALUE_NODES + 1

		const flowJob = (initial: unknown, eventData?: unknown) => ({
			version: 1,
			flow_path: 'f',
			...header,
			jobs: {
				j: {
					initial_job: initial,
					events: eventData ? [{ t: 0, data: eventData }] : []
				}
			}
		})
		// args (a JobArgs row per key) and flow_status.modules (a subtree per entry).
		expect(rejected(flowJob({ id: 'j', args: { a: wide(overBudget) } }))).toBe(true)
		expect(rejected(flowJob({ id: 'j', flow_status: { modules: wide(overBudget) } }))).toBe(true)
		// Same via an event rather than the initial job.
		expect(rejected(flowJob({ id: 'j' }, { flow_status: { modules: wide(overBudget) } }))).toBe(
			true
		)
		// render_all fans out into nested DisplayResults, and the renderer recurses —
		// so a nested budget, not the top-level array's length.
		const nested = Array.from({ length: 400 }, () => ({ render_all: wide(400) }))
		expect(
			rejected(flowJob({ id: 'j' }, { completed: true, job: { id: 'j', result: { render_all: nested } } }))
		).toBe(true)
		// data_tests is a sibling key whose renderer also fans out per entry; nobody
		// had to name it for the budget to cover it.
		expect(
			rejected(
				flowJob({ id: 'j' }, { completed: true, job: { id: 'j', result: { data_tests: wide(overBudget) } } })
			)
		).toBe(true)
		// Depth is its own hazard: a renderer recursing over this blows the stack long
		// before the node count would.
		let deep: unknown = 0
		for (let i = 0; i < MAX_VALUE_DEPTH + 2; i++) deep = { nest: deep }
		expect(rejected(flowJob({ id: 'j', args: { a: deep } }))).toBe(true)

		expect(kindOf(flowJob({ id: 'j', args: { a: 1, b: 'two' } }))).toBe('flow')
	})

	it('bounds what a script and a flow render immediately', () => {
		const wide = (n: number) => Array.from({ length: n }, () => 0)
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
		// SchemaForm recurses into nested properties, so the cap cannot be top-level.
		expect(
			rejected(
				script({
					schema: { properties: { outer: { properties: { inner: wide(MAX_VALUE_NODES + 1) } } } }
				})
			)
		).toBe(true)
		expect(kindOf(script({ args: { a: 1 } }))).toBe('script')

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
		// The module count is the total across the nested tree, not the top-level
		// array's length — a branch or loop body renders nodes just the same.
		expect(
			rejected(
				flowWith({
					modules: [
						{ id: 'loop', value: { type: 'forloopflow', modules: modules(MAX_FLOW_MODULES + 1) } }
					]
				})
			)
		).toBe(true)
		// A branch is a node and an edge even with no modules in it.
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
		// input_transforms renders a table row per entry — a per-module sibling the
		// module count never saw, and the budget covers without naming it.
		expect(
			rejected(
				flowWith({
					modules: [
						{
							id: 'a',
							value: { type: 'rawscript', input_transforms: { x: wide(MAX_VALUE_NODES + 1) } }
						}
					]
				})
			)
		).toBe(true)
		// Notes and groups are rendered overlays; same budget, no separate caps.
		expect(rejected(flowWith({ modules: [], notes: wide(MAX_VALUE_NODES + 1) }))).toBe(true)
		expect(rejected(flowWith({ modules: [], groups: wide(MAX_VALUE_NODES + 1) }))).toBe(true)
		expect(kindOf(flowWith({ modules: modules(3) }))).toBe('flow')
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

	it('bounds an asset sample on its rendered cells, not just per axis', () => {
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
		// Each axis is individually under its cap, and rows of empty objects carry no
		// structure to count, so only the cell-product cap rejects this — which is why
		// the structural budget does not replace it.
		const overBudget = MAX_SAMPLE_CELLS / MAX_SAMPLE_COLUMNS + 1
		expect(rejected(pipeline(sample(overBudget, MAX_SAMPLE_COLUMNS)))).toBe(true)
		expect(kindOf(pipeline(sample(10, 20)))).toBe('pipeline')
	})
})
