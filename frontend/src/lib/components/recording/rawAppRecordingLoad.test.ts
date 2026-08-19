/**
 * `parseRecording` and the per-kind validators are what stand between a `?src=` URL
 * anyone can point the public replay page at and a player that indexes into the
 * payload and renders it. Each rejection below is a way a caller-supplied recording
 * could otherwise reach the DOM or the render loop unbounded.
 */
import { describe, expect, it } from 'vitest'
import {
	MAX_COMPONENT_FANOUT,
	MAX_FLOW_MODULES,
	MAX_RECORDED_JOBS,
	MAX_MAP_ROWS,
	MAX_SAMPLE_CELLS,
	MAX_SERIALIZED_FANOUT_CHARS,
	MAX_TIMELINE_FRAMES,
	MAX_SAMPLE_COLUMNS,
	MAX_VALUE_DEPTH,
	MAX_VALUE_NODES,
	MAX_RECORDING_NODES,
	MAX_VALUE_STRING_CHARS,
	isAppRecording,
	parseRecording
} from './rawAppRecordingLoad'
import { MAX_RECORDED_STEPS, MAX_STEP_TEXT_CHARS } from './rawAppSnapshot'

const valid = () => ({
	version: 1,
	type: 'app',
	recorded_at: '2026-07-25T00:00:00.000Z',
	app_path: 'f/demo/app',
	workspace: 'demo',
	total_duration_ms: 1200,
	viewport: { width: 1280, height: 720 },
	frames: ['<html></html>', '<html><body>x</body></html>'],
	steps: [{ t: 100, kind: 'click', label: 'Clicked button', before: 0, after: 1 }]
})

const rejects: [string, (r: ReturnType<typeof valid>) => unknown][] = [
	['a non-object payload', () => 42],
	['another recording type', (r) => ({ ...r, type: 'flow' })],
	['a future version', (r) => ({ ...r, version: 2 })],
	['a frame that is not a string', (r) => ({ ...r, frames: ['<html></html>', 7] })],
	['a step index past the last frame', (r) => ({ ...r, steps: [{ ...r.steps[0], after: 9 }] })],
	['a negative step index', (r) => ({ ...r, steps: [{ ...r.steps[0], before: -1 }] })],
	['an unknown interaction kind', (r) => ({ ...r, steps: [{ ...r.steps[0], kind: 'exec' }] })],
	['a NaN timestamp', (r) => ({ ...r, steps: [{ ...r.steps[0], t: NaN }] })],
	[
		'a label past the text budget',
		(r) => ({ ...r, steps: [{ ...r.steps[0], label: 'x'.repeat(MAX_STEP_TEXT_CHARS + 1) }] })
	],
	[
		'more steps than the recorder can produce',
		(r) => ({
			...r,
			steps: Array.from({ length: MAX_RECORDED_STEPS + 1 }, () => ({ ...r.steps[0] }))
		})
	],
	['a non-numeric viewport', (r) => ({ ...r, viewport: { width: '1280px', height: 720 } })],
	['a viewport beyond any screen', (r) => ({ ...r, viewport: { width: 1e6, height: 720 } })],
	['a negative duration', (r) => ({ ...r, total_duration_ms: -1 })],
	['a missing recorded_at', (r) => ({ ...r, recorded_at: undefined })]
]

describe('isAppRecording', () => {
	it('accepts a well-formed recording', () => {
		expect(isAppRecording(valid())).toBe(true)
		// Steps may legitimately lack a snapshot the recorder had to skip.
		expect(isAppRecording({ ...valid(), steps: [{ t: 1, kind: 'click', label: 'x' }] })).toBe(true)
	})

	it.each(rejects)('rejects %s', (_name, mutate) => {
		expect(isAppRecording(mutate(valid()))).toBe(false)
	})
})

// A recorded job is a completed job as the API returns it.
const job = (extra: Record<string, unknown> = {}) => ({
	id: 'j',
	type: 'CompletedJob',
	...extra
})

const header = { recorded_at: '2026-07-25T00:00:00.000Z', total_duration_ms: 1000 }

const flowRec = (extra: Record<string, unknown> = {}) => ({
	version: 2,
	flow_path: 'f',
	...header,
	root_job_id: 'j',
	jobs: { j: job() },
	...extra
})

const scriptRec = (extra: Record<string, unknown> = {}) => ({
	version: 2,
	type: 'script',
	script_path: 's',
	...header,
	code: 'echo hi',
	language: 'bash',
	job: job(),
	...extra
})

const pipeline = (extra: Record<string, unknown> = {}) => ({
	version: 2,
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
		expect(kindOf(flowRec())).toBe('flow')
		expect(kindOf(flowRec({ type: 'flow' }))).toBe('flow')
		expect(kindOf(scriptRec())).toBe('script')
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

	it('upgrades v1 job recordings in place, refusing only what cannot be salvaged', () => {
		// v1 stored live event streams; published recordings (the hub) collapse to
		// the completed job each stream carried and keep replaying.
		const v1Stream = (j: Record<string, unknown>, logChunks: string[] = []) => ({
			initial_job: { id: j.id },
			events: [
				...logChunks.map((c, i) => ({ t: i, data: { new_logs: c, log_offset: i + 1 } })),
				{ t: 5, data: { completed: true, job: j } }
			]
		})
		const v1Flow = {
			version: 1,
			flow_path: 'f',
			...header,
			jobs: {
				root: v1Stream(job({ id: 'root', job_kind: 'flowpreview' })),
				// The backfilled completed job carried no logs — only the stream did.
				sub: v1Stream(job({ id: 'sub', parent_job: 'root' }), ['line 1\n', 'line 2\n'])
			}
		}
		const flow = parseRecording(v1Flow)
		expect(flow.ok && flow.loaded.kind).toBe('flow')
		expect(flow.ok && (flow.loaded.recording as any).root_job_id).toBe('root')
		expect(flow.ok && (flow.loaded.recording as any).jobs.sub.logs).toBe('line 1\nline 2\n')
		const v1Script = {
			version: 1,
			type: 'script',
			script_path: 's',
			...header,
			code: 'x',
			language: 'bash',
			job: v1Stream(job())
		}
		expect(kindOf(v1Script)).toBe('script')
		// A v1 file whose streams carry no completed job has nothing to replay from.
		const v1Empty = parseRecording({ version: 1, flow_path: 'f', ...header, jobs: {} })
		expect(v1Empty.ok ? '' : v1Empty.error).toMatch(/older version/)
		const future = parseRecording(flowRec({ version: 3 }))
		expect(future.ok ? '' : future.error).toMatch(/newer version/)
		// App recordings never changed format and stay at version 1.
		const futureApp = parseRecording({ ...valid(), version: 2 })
		expect(futureApp.ok ? '' : futureApp.error).toMatch(/newer version/)
	})

	it('rejects payloads a player would choke on rather than falling through to flow', () => {
		// A `?src=` origin is arbitrary: a claimed kind must be validated as that
		// kind, never silently reinterpreted.
		expect(rejected({ version: 2, type: 'script', code: 'x', language: 'bash' })).toBe(true)
		expect(rejected({ version: 2, type: 'pipeline', timeline: [], jobs: {} })).toBe(true)
		expect(rejected({ version: 2, type: 'somethingelse', jobs: {} })).toBe(true)
		expect(rejected(null)).toBe(true)
		expect(rejected([{ version: 2 }])).toBe(true)
		// Header fields are required by the types and rendered by every player.
		expect(rejected({ version: 2, root_job_id: 'j', jobs: { j: job() } })).toBe(true)
		expect(
			rejected({
				version: 2,
				flow_path: 'f',
				recorded_at: 'x',
				root_job_id: 'j',
				jobs: { j: job() }
			})
		).toBe(true)
		// The replay is keyed and anchored by job ids, so a job without one — or a
		// root id naming no recorded job — has nothing to attach to.
		expect(rejected(flowRec({ jobs: { j: { type: 'CompletedJob' } } }))).toBe(true)
		expect(rejected(flowRec({ root_job_id: 'missing' }))).toBe(true)
		// Cardinality, not just shape: each job mounts a JobLoader.
		const tooManyJobs = Object.fromEntries(
			Array.from({ length: MAX_RECORDED_JOBS + 1 }, (_, i) => [`j${i}`, job({ id: `j${i}` })])
		)
		expect(rejected(flowRec({ jobs: { j: job(), ...tooManyJobs } }))).toBe(true)
	})

	it('holds every recorded value to one structural render budget', () => {
		// One rule, asserted over the shapes it has to hold for: a value a component
		// expands eagerly may not carry more than MAX_VALUE_NODES of structure, at any
		// depth and under any key.
		const wide = (n: number) => Array.from({ length: n }, () => 0)
		const overBudget = MAX_VALUE_NODES + 1

		const flowJob = (jobExtra: Record<string, unknown>) => flowRec({ jobs: { j: job(jobExtra) } })
		// args (a JobArgs row per key) and flow_status.modules (a subtree per entry).
		expect(rejected(flowJob({ args: { a: wide(overBudget) } }))).toBe(true)
		expect(rejected(flowJob({ flow_status: { modules: wide(overBudget) } }))).toBe(true)
		// render_all fans out into nested DisplayResults, and the renderer recurses —
		// so a nested budget, not the top-level array's length.
		const nested = Array.from({ length: 400 }, () => ({ render_all: wide(400) }))
		expect(rejected(flowJob({ result: { render_all: nested } }))).toBe(true)
		// data_tests is a sibling key whose renderer also fans out per entry; nobody
		// had to name it for the budget to cover it.
		expect(rejected(flowJob({ result: { data_tests: wide(overBudget) } }))).toBe(true)
		// Depth is its own hazard: a renderer recursing over this blows the stack long
		// before the node count would.
		let deep: unknown = 0
		for (let i = 0; i < MAX_VALUE_DEPTH + 2; i++) deep = { nest: deep }
		expect(rejected(flowJob({ args: { a: deep } }))).toBe(true)

		expect(kindOf(flowJob({ args: { a: 1, b: 'two' } }))).toBe('flow')
	})

	it('bounds what a script and a flow render immediately', () => {
		const wide = (n: number) => Array.from({ length: n }, () => 0)
		// SchemaForm recurses into nested properties, so the cap cannot be top-level.
		expect(
			rejected(
				scriptRec({
					schema: { properties: { outer: { properties: { inner: wide(MAX_VALUE_NODES + 1) } } } }
				})
			)
		).toBe(true)
		expect(kindOf(scriptRec({ args: { a: 1 } }))).toBe('script')
		// `args` and `schema.properties` arrive as the root of the walk on this kind.
		const wideMap = Object.fromEntries(
			Array.from({ length: MAX_MAP_ROWS + 1 }, (_, i) => [`a${i}`, 1])
		)
		expect(rejected(scriptRec({ args: wideMap }))).toBe(true)
		expect(rejected(scriptRec({ args: Array.from({ length: MAX_MAP_ROWS + 1 }, () => 0) }))).toBe(
			true
		)
		expect(rejected(scriptRec({ schema: { properties: wideMap } }))).toBe(true)

		const flowWith = (value: unknown) => flowRec({ flow: { value } })
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
		// Notes and groups each mount a graph node, so they are capped by component
		// fan-out and not merely by structure: minimal entries are cheap enough that
		// the node budget would admit tens of thousands of them.
		expect(rejected(flowWith({ modules: [], notes: wide(MAX_COMPONENT_FANOUT + 1) }))).toBe(true)
		expect(rejected(flowWith({ modules: [], groups: wide(MAX_COMPONENT_FANOUT + 1) }))).toBe(true)
		// Split across both, since one graph draws them together.
		expect(
			rejected(
				flowWith({
					modules: [],
					notes: wide(MAX_COMPONENT_FANOUT),
					groups: wide(MAX_COMPONENT_FANOUT)
				})
			)
		).toBe(true)
		expect(kindOf(flowWith({ modules: [], notes: wide(10), groups: wide(10) }))).toBe('flow')
		expect(kindOf(flowWith({ modules: modules(3) }))).toBe('flow')
	})

	it('bounds the three things that cost differently: structure, text, components', () => {
		const wide = (n: number) => Array.from({ length: n }, () => 0)
		const flowJob = (result: unknown) => flowRec({ jobs: { j: job({ result }) } })

		// Fan-out is cumulative across the value, not per array: nesting sidesteps a
		// per-array cap while mounting a component per leaf.
		const side = 40
		expect(side * side).toBeGreaterThan(MAX_COMPONENT_FANOUT)
		expect(
			rejected(
				flowJob({ render_all: Array.from({ length: side }, () => ({ render_all: wide(side) })) })
			)
		).toBe(true)

		// Component fan-out: well inside the node budget, but one nested DisplayResult
		// per entry is orders of magnitude more expensive than a node.
		const fanout = MAX_COMPONENT_FANOUT + 1
		expect(fanout).toBeLessThan(MAX_VALUE_NODES)
		expect(rejected(flowJob({ render_all: wide(fanout) }))).toBe(true)
		expect(rejected(flowJob({ data_tests: wide(fanout) }))).toBe(true)
		// ...including the serialized form, since a string is one node however many
		// components the renderer parses out of it.
		expect(rejected(flowJob({ data_tests: JSON.stringify(wide(fanout)) }))).toBe(true)
		expect(rejected(flowJob({ error: { data_tests: JSON.stringify(wide(fanout)) } }))).toBe(true)
		expect(kindOf(flowJob({ render_all: wide(10) }))).toBe('flow')

		// Text: a long string is one node, so only the character budget sees it.
		expect(rejected(flowJob({ big: 'x'.repeat(MAX_VALUE_STRING_CHARS + 1) }))).toBe(true)
		// A flat map rendered one row per entry: cheap per node, so the node budget
		// alone lets ~100k rows through.
		const rows = (n: number) =>
			Object.fromEntries(Array.from({ length: n }, (_, i) => [`a${i}`, 1]))
		expect(MAX_MAP_ROWS + 1).toBeLessThan(MAX_VALUE_NODES)
		expect(rejected(flowJob({ args: rows(MAX_MAP_ROWS + 1) }))).toBe(true)
		// `args` is only conventionally a map: an array gets a row per entry too.
		expect(rejected(flowJob({ args: Array.from({ length: MAX_MAP_ROWS + 1 }, () => 0) }))).toBe(
			true
		)
		expect(kindOf(flowJob({ args: rows(5) }))).toBe('flow')
		// An object *key* is rendered text too (JobArgs prints it), and charging only
		// values would let a huge key through.
		expect(rejected(flowJob({ ['k'.repeat(MAX_VALUE_STRING_CHARS + 1)]: 1 }))).toBe(true)
		// A serialized fan-out larger than validation is willing to decode is refused
		// rather than measured — decoding it to size it would be the DoS.
		expect(
			rejected(
				flowJob({
					data_tests: JSON.stringify([
						{ test: 't', violating: 1, pad: 'x'.repeat(MAX_SERIALIZED_FANOUT_CHARS) }
					])
				})
			)
		).toBe(true)
	})

	it('budgets the whole flow object, not just its value', () => {
		// `schema` renders through SchemaViewer (Input Schema tab, Input node) exactly
		// as `value` renders through the graph, so the budget belongs one level up.
		const properties = Object.fromEntries(
			Array.from({ length: MAX_VALUE_NODES }, (_, i) => [`p${i}`, { type: 'string' }])
		)
		const res = parseRecording(
			flowRec({ flow: { schema: { properties }, value: { modules: [] } } })
		)
		expect(res.ok).toBe(false)
		// A module's inline `content` is the flow kind's equivalent of a script's
		// `code`, and text is not structure.
		expect(
			rejected(
				flowRec({
					flow: {
						value: {
							modules: [
								{
									id: 'a',
									value: { type: 'rawscript', content: 'x'.repeat(MAX_VALUE_STRING_CHARS + 1) }
								}
							]
						}
					}
				})
			)
		).toBe(true)
	})

	it('bounds graph contents, rendered metadata strings and scheduled timers', () => {
		// The canvas emits a node and edge per nested graph entry, so the four
		// top-level array lengths were never the bound.
		expect(
			rejected(
				pipeline({
					graph: {
						runnables: [
							{
								path: 'f/a/b',
								usage_kind: 'script',
								data_tests: Array.from({ length: MAX_VALUE_NODES }, (_, i) => ({ test: `t${i}` }))
							}
						],
						assets: [],
						edges: [],
						triggers: []
					}
				})
			)
		).toBe(true)

		// Metadata strings land in a header, a highlighter class or an error panel, and
		// were only `typeof === 'string'`.
		const long = 'x'.repeat(5000)
		expect(rejected(scriptRec({ language: long }))).toBe(true)
		expect(rejected(pipeline({ codes: { 'f/a/b': { content: 'x', language: long } } }))).toBe(true)
		expect(
			rejected(
				pipeline({
					assetSamples: { 'ducklake:main/t': { kind: 'ducklake', path: 'main/t', error: long } }
				})
			)
		).toBe(true)

		// A pipeline's timeline frames all become timers in a single pass too.
		expect(
			rejected(
				pipeline({
					timeline: Array.from({ length: MAX_TIMELINE_FRAMES + 1 }, () => ({ t: 0, statuses: {} }))
				})
			)
		).toBe(true)
	})

	it('refuses a huge recording before it looks at what any field means', () => {
		// The backstop needs no key name, so a field no validator mentions is still
		// bounded.
		const filler = Array.from({ length: 5000 }, () => Array.from({ length: 500 }, () => 0))
		const res = parseRecording(flowRec({ some_future_field: filler }))
		expect(res.ok).toBe(false)
		expect(res.ok ? '' : res.error).toMatch(new RegExp(`more than ${MAX_RECORDING_NODES} values`))

		// Structure hidden behind wrappers deeper than the ceiling must be refused, not
		// silently uncounted — a backstop that gives up is not a backstop.
		let buried: unknown = filler
		for (let i = 0; i < MAX_VALUE_DEPTH + 2; i++) buried = { nest: buried }
		expect(rejected(flowRec({ buried }))).toBe(true)
	})

	it('bounds an errored asset sample, which still renders its own fields', () => {
		// The error branch shows the message instead of the table, but `uri` and
		// `rowCount` render either way.
		expect(
			rejected(
				pipeline({
					assetSamples: {
						'ducklake:main/t': {
							kind: 'ducklake',
							path: 'main/t',
							error: 'table missing',
							uri: 'x'.repeat(MAX_VALUE_STRING_CHARS + 1)
						}
					}
				})
			)
		).toBe(true)
		expect(
			kindOf(
				pipeline({
					assetSamples: {
						'ducklake:main/t': {
							kind: 'ducklake',
							path: 'main/t',
							uri: 'ducklake://main/t',
							error: 'table missing'
						}
					}
				})
			)
		).toBe('pipeline')
	})

	it('tells an oversized recording apart from a corrupt one', () => {
		// A wide for-loop flow records a job per iteration, so a real capture can trip
		// this — it must not read as "your recorder wrote a broken file".
		const tooManyJobs = Object.fromEntries(
			Array.from({ length: MAX_RECORDED_JOBS + 1 }, (_, i) => [`j${i}`, job({ id: `j${i}` })])
		)
		const res = parseRecording(flowRec({ jobs: { j: job(), ...tooManyJobs } }))
		expect(res.ok ? '' : res.error).toMatch(/holds \d+ jobs/)
		const corrupt = parseRecording(flowRec({ jobs: 'nope' }))
		expect(corrupt.ok ? '' : corrupt.error).toBe('Invalid flow recording format.')
		// The render budget is the cap a real capture is most likely to trip, so it
		// names the value and what about it was too big.
		const fat = parseRecording(
			flowRec({
				jobs: {
					j: job({
						result: { render_all: Array.from({ length: MAX_COMPONENT_FANOUT + 1 }, () => 0) }
					})
				}
			})
		)
		expect(fat.ok ? '' : fat.error).toMatch(/a recorded job carries more than \d+ `render_all`/)
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
