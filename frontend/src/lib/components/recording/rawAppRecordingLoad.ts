/**
 * Validation for a recording arriving from outside — an uploaded file or a
 * `?src=` URL. Both replay pages go through this: a recording is caller-supplied
 * data that a player indexes into and renders per step, so its shape, its
 * cardinality and its sizes all have to hold before anything mounts.
 *
 * Covers every kind of recording, not only raw-app ones: the file name is fixed by
 * `package.json`'s `./components/recording/rawAppRecordingLoad` export, which the hub
 * imports, so renaming it means changing the hub in the same breath.
 */
import type { Job } from '$lib/gen'
import type { FlowRecording, PipelineRecording, RawAppRecording, ScriptRecording } from './types'
import {
	MAX_RECORDED_STEPS,
	MAX_STEP_TEXT_CHARS,
	MAX_TOTAL_FRAME_CHARS,
	RAW_APP_INTERACTION_KINDS,
	type RawAppInteractionKind
} from './rawAppSnapshot'

/** Upper bound on a fetched recording (JSON of capped frames) so an arbitrary
 * origin can't exhaust the tab before validation runs. */
export const MAX_RECORDING_BYTES = 100 * 1024 * 1024

/** Total structure in the whole recording, applied before any per-kind validator.
 * The per-value budgets below each cover only a field they name, so a field added
 * later is unbounded until someone names it; this one needs no field, which is what
 * makes the set closed. Far above any real capture, far below what a tab survives. */
export const MAX_RECORDING_NODES = 2_000_000
/* Cap on the job-based recordings (flow/script/pipeline): each recorded job
 * mounts a JobLoader whose synthesized replay stream costs `setTimeout`s created
 * up front (the per-job event count is capped by the synthesis itself, in
 * `replayStream.ts`), so the job count — not just the byte size — decides
 * whether the tab survives. */
export const MAX_RECORDED_JOBS = 2000
/** The backstop: structure one recorded value may expand into, where a value is
 * what a component renders eagerly. Deliberately generous — it is not the precise
 * bound but the one that catches keys nobody has named; anything mounting a
 * component per entry costs far more and gets {@link MAX_COMPONENT_FANOUT} on top. */
export const MAX_VALUE_NODES = 100_000
/** Total characters of text in one value. Strings are one node however long they
 * are, so this is the part of "how big is this value" the node count structurally
 * cannot see: a flow module's inline `content`/`lock` is syntax-highlighted in one
 * pass, exactly like the `code` that {@link MAX_CODE_CHARS} already covers. */
export const MAX_VALUE_STRING_CHARS = 8 * 1024 * 1024
/** Depth is its own hazard: a renderer recursing over a deeply nested value blows
 * the stack long before the node count matters. Well above real data (recursive
 * results nest tens deep) and well below what overflows a JS stack. */
export const MAX_VALUE_DEPTH = 256
/** Entries whose renderer mounts a *component* each rather than a cell or a row —
 * `render_all`, `data_tests`, and a flow's graph overlays. Two orders of magnitude
 * below the node budget, because that is roughly the cost ratio. Counted through the
 * serialized form too: a JSON string is one node whatever it decodes to. */
export const MAX_COMPONENT_FANOUT = 1000
/** How much serialized text a fan-out collection may be before validation stops
 * measuring it and just refuses. A real checklist is a few KB; anything near the text
 * budget would cost hundreds of MB to decode, and decoding it to find out how big it
 * is would be the denial of service. */
export const MAX_SERIALIZED_FANOUT_CHARS = 256 * 1024
/** Entries in one flat map a renderer turns into a row each. This and
 * {@link MAX_COMPONENT_FANOUT} are lists of keys, so they are inherently incomplete:
 * cost depends on the renderer, not the shape. Add the key of any new renderer that
 * iterates a recorded collection — {@link MAX_RECORDING_NODES} alone is too coarse. */
export const MAX_MAP_ROWS = 2000
/** `PipelineRecordingReplay.startReplay` schedules every frame in one pass, so this
 * counts timers created at once — frames at `t: 0` all land in the same tick, and
 * each reassigns the whole per-node status map and rebuilds the derived id/state maps
 * over its entire key set. */
export const MAX_TIMELINE_FRAMES = 5000
export const MAX_FRAME_STATUSES = 5000
/** Graph elements each become a rendered canvas node or edge. */
export const MAX_GRAPH_ELEMENTS = 2000
/* An asset sample renders as a `rows × columns` table of plain `<td>`s, so the
 * product is what costs, and the per-axis caps alone would allow millions of
 * cells from a tiny payload of empty row objects. */
export const MAX_SAMPLE_ROWS = 5000
export const MAX_SAMPLE_COLUMNS = 500
/** Not subsumed by {@link MAX_VALUE_NODES}: the table is the cross product of two
 * independent arrays, so rows of *empty* objects carry no structure to count yet
 * still render a cell each. Structure and cross products are different bounds. */
export const MAX_SAMPLE_CELLS = 100_000
/** Captured source is syntax-highlighted in one pass. */
export const MAX_CODE_CHARS = 4 * 1024 * 1024
/** `FlowGraphV2` builds and lays out a node per module, recursing into branches and
 * loops, plus one per note or group. Kept alongside the render budget because it
 * gives the flow-specific count a name; the budget is what makes it exhaustive. */
export const MAX_FLOW_MODULES = 5000

const isObject = (v: unknown): v is Record<string, unknown> =>
	typeof v === 'object' && v !== null && !Array.isArray(v)

const isShortText = (v: unknown, required = false) =>
	required
		? typeof v === 'string' && v.length <= MAX_STEP_TEXT_CHARS
		: v === undefined || (typeof v === 'string' && v.length <= MAX_STEP_TEXT_CHARS)

const isSize = (v: unknown) => typeof v === 'number' && Number.isFinite(v) && v > 0 && v <= 20000

const isBoundedCode = (v: unknown) => typeof v === 'string' && v.length <= MAX_CODE_CHARS

const isBoundedArray = (v: unknown, max: number): v is unknown[] =>
	Array.isArray(v) && v.length <= max

const isObjectArray = (v: unknown, max: number) => isBoundedArray(v, max) && v.every(isObject)

/** True when `data` is a well-formed app recording this build can replay. */
export function isAppRecording(data: unknown): data is RawAppRecording {
	if (!isObject(data) || data.version !== 1 || data.type !== 'app') return false
	// Every frame is parsed and re-serialized before the iframe parses it again,
	// so a single huge frame would freeze the tab even under the download cap.
	const validFrames =
		Array.isArray(data.frames) &&
		data.frames.length <= 2 * MAX_RECORDED_STEPS + 1 &&
		data.frames.every((f) => typeof f === 'string') &&
		(data.frames as string[]).reduce((sum, f) => sum + f.length, 0) <= MAX_TOTAL_FRAME_CHARS
	if (!validFrames) return false
	const frameCount = (data.frames as string[]).length
	// An index must address a frame that exists; `undefined` stays legitimate for
	// a capture the recorder had to skip.
	const isIndex = (v: unknown) =>
		v === undefined || (typeof v === 'number' && Number.isInteger(v) && v >= 0 && v < frameCount)
	const validSteps =
		Array.isArray(data.steps) &&
		data.steps.length <= MAX_RECORDED_STEPS &&
		data.steps.every(
			(s: unknown) =>
				isObject(s) &&
				// Finite, not merely numeric: the timeline positions each checkpoint at
				// `t / total_duration_ms`, and a NaN there places nothing.
				Number.isFinite(s.t) &&
				RAW_APP_INTERACTION_KINDS.includes(s.kind as RawAppInteractionKind) &&
				isShortText(s.label, true) &&
				isShortText(s.target) &&
				isShortText(s.selector) &&
				isShortText(s.value) &&
				isIndex(s.before) &&
				isIndex(s.after)
		)
	// The viewport lands in the snapshot iframe's `style`, and the duration is
	// divided into every step timestamp by the timeline.
	const validViewport =
		isObject(data.viewport) && isSize(data.viewport.width) && isSize(data.viewport.height)
	const validDuration =
		typeof data.total_duration_ms === 'number' &&
		Number.isFinite(data.total_duration_ms) &&
		data.total_duration_ms >= 0
	const validHeader =
		isShortText(data.app_path) && isShortText(data.workspace) && isShortText(data.recorded_at, true)
	return validSteps && validViewport && validDuration && validHeader
}

/** Keys whose renderer mounts a component per entry. See {@link MAX_COMPONENT_FANOUT}. */
const COMPONENT_FANOUT_KEYS = ['render_all', 'data_tests']

/** Keys holding a flat map rendered as one row per entry: `args` (`JobArgs` sorts the
 * keys and mounts a row each) and a schema's `properties` (`SchemaForm`/`SchemaViewer`
 * a field each). Capped per collection rather than cumulatively: a hundred small
 * schemas nested in a flow are fine, one map of 90k keys is not. */
const MAP_ROW_KEYS = ['args', 'properties']

/** Entry count of a fan-out collection, decoding the serialized form renderers
 * accept — a JSON string is one node however many components it expands into. Only
 * decodes what could be a legitimate checklist: parsing megabytes here would commit
 * the very allocation this prevents, so anything larger is refused unmeasured. */
function fanoutLength(v: unknown): number {
	if (Array.isArray(v)) return v.length
	if (typeof v === 'string') {
		if (v.length > MAX_SERIALIZED_FANOUT_CHARS) return Infinity
		try {
			const decoded = JSON.parse(v)
			return Array.isArray(decoded) ? decoded.length : 0
		} catch {
			return 0
		}
	}
	return 0
}

/** Entries a renderer would turn into rows, for {@link MAX_MAP_ROWS}. Counts arrays
 * as well as objects: `args` is only *conventionally* a map, and an array of 90k
 * primitives costs one node each and gets a row each just the same. */
const rowCount = (v: unknown) =>
	Array.isArray(v) ? v.length : isObject(v) ? Object.keys(v).length : 0

/** Why one recorded value is too big to render, or `undefined`. Three bounds
 * because none subsumes the others: structure misses a 60MB string, text misses a
 * collection that mounts a component per entry, and fan-out misses everything
 * unnamed. Bails on the first blown bound so the walk is never itself the attack. */
function describeValueOverflow(
	v: unknown,
	budget = { nodes: MAX_VALUE_NODES, chars: MAX_VALUE_STRING_CHARS, fanout: MAX_COMPONENT_FANOUT },
	depth = 0
): string | undefined {
	if (depth > MAX_VALUE_DEPTH) return `nested more than ${MAX_VALUE_DEPTH} levels deep`
	if (typeof v === 'string') {
		budget.chars -= v.length
		return budget.chars < 0 ? `more than ${MAX_VALUE_STRING_CHARS} characters of text` : undefined
	}
	if (Array.isArray(v)) {
		budget.nodes -= v.length
		if (budget.nodes < 0) return `more than ${MAX_VALUE_NODES} values to render`
		for (const item of v) {
			const over = describeValueOverflow(item, budget, depth + 1)
			if (over) return over
		}
		return undefined
	}
	if (isObject(v)) {
		const keys = Object.keys(v)
		budget.nodes -= keys.length
		if (budget.nodes < 0) return `more than ${MAX_VALUE_NODES} values to render`
		for (const k of keys) {
			// A key is rendered text as much as a value is — `JobArgs` prints it — so it
			// is charged against the same budget.
			budget.chars -= k.length
			if (budget.chars < 0) return `more than ${MAX_VALUE_STRING_CHARS} characters of text`
			// Cumulative across the value, not per array: `render_all` nests, so 300
			// arrays of 300 are 90k components with no single array over the cap.
			if (COMPONENT_FANOUT_KEYS.includes(k)) {
				budget.fanout -= fanoutLength(v[k])
				if (budget.fanout < 0) {
					return `more than ${MAX_COMPONENT_FANOUT} \`${k}\`-style entries, each of which mounts its own component`
				}
			}
			if (MAP_ROW_KEYS.includes(k) && rowCount(v[k]) > MAX_MAP_ROWS) {
				return `a \`${k}\` of more than ${MAX_MAP_ROWS} entries, each of which renders a row`
			}
			const over = describeValueOverflow(v[k], budget, depth + 1)
			if (over) return over
		}
		return undefined
	}
	return undefined
}

/** Structure in the whole recording, for the {@link MAX_RECORDING_NODES} backstop.
 * Separate from {@link describeValueOverflow} because it deliberately knows nothing
 * about keys or renderers — it just refuses to let an arbitrary payload be huge. */
function countRecordingNodes(v: unknown, budget = { n: MAX_RECORDING_NODES + 1 }): number {
	let count = 0
	// Past the ceiling the answer is `Infinity`, not the partial count: giving up on
	// the walk and reporting what was seen so far would let any amount of structure
	// hide behind a few hundred wrappers, which is the opposite of a backstop.
	let tooDeep = false
	const walk = (x: unknown, depth: number) => {
		if (tooDeep || budget.n <= 0) return
		if (depth > MAX_VALUE_DEPTH) {
			tooDeep = true
			return
		}
		if (Array.isArray(x)) {
			count += x.length
			budget.n -= x.length
			for (const i of x) walk(i, depth + 1)
		} else if (isObject(x)) {
			const keys = Object.keys(x)
			count += keys.length
			budget.n -= keys.length
			for (const k of keys) walk(x[k], depth + 1)
		}
	}
	walk(v, 0)
	return tooDeep ? Infinity : count
}

/** True when one recorded value is renderable. Apply this to each value a component
 * expands eagerly (a job's args/result/flow_status, a flow definition, an asset
 * sample); the *number* of such values is bounded separately. */
const withinRenderBudget = (v: unknown) => describeValueOverflow(v) === undefined

/** A recorded completed job. The synthesized replay walks its timestamps and
 * flow status and renders everything hanging off it (args, result, logs), so it
 * is held to the render budget at any depth; `id` keys it into the replay. */
function isRecordedJob(j: unknown): j is Job {
	return isObject(j) && isShortText(j.id, true) && withinRenderBudget(j)
}

/** The `jobs` map the flow and pipeline recordings carry. */
function isJobsMap(v: unknown): v is Record<string, Job> {
	if (!isObject(v)) return false
	const jobs = Object.values(v)
	if (jobs.length > MAX_RECORDED_JOBS) return false
	return jobs.every(isRecordedJob)
}

/** The header every recording renders: a title, a `recorded_at` each player parses
 * as a date, and a duration it divides by. Required by the types, so a payload
 * missing them shows `Invalid Date` and `NaN` instead. */
function hasValidHeader(data: Record<string, unknown>, pathField: string): boolean {
	return (
		isShortText(data[pathField], true) &&
		isShortText(data.recorded_at, true) &&
		typeof data.total_duration_ms === 'number' &&
		Number.isFinite(data.total_duration_ms) &&
		data.total_duration_ms >= 0
	)
}

/** Total modules across a flow definition's nested structure (branches, loops),
 * stopping as soon as the budget is blown so a hostile tree can't make the walk
 * itself the denial of service. */
function countFlowModules(modules: unknown, budget: number): number {
	if (!Array.isArray(modules)) return 0
	let n = 0
	for (const m of modules) {
		if (++n > budget) return n
		if (!isObject(m) || !isObject(m.value)) continue
		for (const key of ['modules', 'default'] as const) {
			n += countFlowModules((m.value as Record<string, unknown>)[key], budget - n)
			if (n > budget) return n
		}
		const branches = (m.value as Record<string, unknown>).branches
		if (Array.isArray(branches)) {
			for (const b of branches) {
				// The branch itself is a node and an edge even when it holds no modules,
				// so counting only its contents would let empty branches ride free.
				if (++n > budget) return n
				if (!isObject(b)) continue
				n += countFlowModules(b.modules, budget - n)
				if (n > budget) return n
			}
		}
	}
	return n
}

/** True when `data` is a well-formed script recording. */
export function isScriptRecording(data: unknown): data is ScriptRecording {
	if (!isObject(data) || data.version !== 2 || data.type !== 'script') return false
	// `code` is highlighted in one pass and `language` selects the grammar.
	return (
		hasValidHeader(data, 'script_path') &&
		isRecordedJob(data.job) &&
		isBoundedCode(data.code) &&
		// Selects a highlighter grammar and is rendered in the player's header.
		isShortText(data.language, true) &&
		// These arrive as the root of the walk, where there is no enclosing key for
		// MAP_ROW_KEYS to match, so their own row counts are checked here.
		rowCount(data.args) <= MAX_MAP_ROWS &&
		rowCount((data.schema as Record<string, unknown> | undefined)?.properties) <= MAX_MAP_ROWS &&
		withinRenderBudget(data.schema) &&
		withinRenderBudget(data.args)
	)
}

/** True when `data` is a well-formed pipeline recording. */
export function isPipelineRecording(data: unknown): data is PipelineRecording {
	if (!isObject(data) || data.version !== 2 || data.type !== 'pipeline') return false
	const g = data.graph
	const validGraph =
		isObject(g) &&
		// The canvas emits a node and an edge per nested entry too (a runnable's custom
		// `data_tests`, its column lineage), so the whole graph goes through the budget
		// rather than just the lengths of the four top-level arrays.
		withinRenderBudget(g) &&
		isObjectArray(g.runnables, MAX_GRAPH_ELEMENTS) &&
		isObjectArray(g.assets, MAX_GRAPH_ELEMENTS) &&
		isObjectArray(g.edges, MAX_GRAPH_ELEMENTS) &&
		isBoundedArray(g.triggers, MAX_GRAPH_ELEMENTS) &&
		g.triggers.every((t) => isObject(t) && typeof t.trigger_kind === 'string') &&
		(g.macro_edges === undefined || isObjectArray(g.macro_edges, MAX_GRAPH_ELEMENTS)) &&
		(g.test_edges === undefined || isObjectArray(g.test_edges, MAX_GRAPH_ELEMENTS))
	const validTimeline =
		isBoundedArray(data.timeline, MAX_TIMELINE_FRAMES) &&
		data.timeline.every(
			(f) =>
				isObject(f) &&
				isObject(f.statuses) &&
				Object.keys(f.statuses).length <= MAX_FRAME_STATUSES &&
				withinRenderBudget(f.statuses) &&
				Object.values(f.statuses).every(isObject)
		)
	// A sample renders `rows`/`columns` unless it carries a non-empty `error`.
	const validSamples =
		data.assetSamples === undefined ||
		(isObject(data.assetSamples) &&
			Object.values(data.assetSamples).every(
				(s) =>
					isObject(s) &&
					// Both branches render the sample's own fields (`uri`, `rowCount`), so the
					// budget applies either way; only the table is extra.
					withinRenderBudget(s) &&
					((isShortText(s.error, true) && s.error !== '') ||
						(isObjectArray(s.rows, MAX_SAMPLE_ROWS) &&
							isObjectArray(s.columns, MAX_SAMPLE_COLUMNS) &&
							(s.rows as unknown[]).length * (s.columns as unknown[]).length <= MAX_SAMPLE_CELLS))
			))
	const validCodes =
		data.codes === undefined ||
		(isObject(data.codes) &&
			Object.values(data.codes).every(
				(c) => isObject(c) && isBoundedCode(c.content) && isShortText(c.language, true)
			))
	return (
		hasValidHeader(data, 'folder') &&
		validGraph &&
		validTimeline &&
		isJobsMap(data.jobs) &&
		validSamples &&
		validCodes
	)
}

/** True when `data` is a well-formed flow recording. `type` is absent on
 * recordings taken before the discriminator existed. */
export function isFlowRecording(data: unknown): data is FlowRecording {
	if (!isObject(data) || data.version !== 2) return false
	if (data.type !== undefined && data.type !== 'flow') return false
	if (!hasValidHeader(data, 'flow_path') || !isJobsMap(data.jobs)) return false
	// The player anchors the whole replay on the root job, so it must exist.
	if (typeof data.root_job_id !== 'string' || !isObject(data.jobs[data.root_job_id])) return false
	if (data.flow === undefined) return true
	// The player hands the whole `flow` to FlowViewer, so `schema` renders (Input
	// Schema tab, Input node) just like `value` does — budget one level up.
	if (!isObject(data.flow) || !withinRenderBudget(data.flow)) return false
	const value = data.flow.value
	if (value === undefined) return true
	if (!isObject(value)) return false
	// The graph mounts a node per note and per group alongside the modules, so they
	// fan out like `render_all` does — the structural budget alone would admit tens
	// of thousands of minimal entries and lock the tab before Play.
	if (fanoutLength(value.notes) + fanoutLength(value.groups) > MAX_COMPONENT_FANOUT) return false
	return countFlowModules(value.modules, MAX_FLOW_MODULES) <= MAX_FLOW_MODULES
}

/** A recording that passed validation, tagged with the player it needs. */
export type LoadedRecording =
	| { kind: 'app'; recording: RawAppRecording }
	| { kind: 'script'; recording: ScriptRecording }
	| { kind: 'pipeline'; recording: PipelineRecording }
	| { kind: 'flow'; recording: FlowRecording }

/** The cap a well-formed but oversized recording tripped, if any. A genuine
 * capture can hit these (a wide for-loop flow records a job per iteration), so it
 * must not be reported with the same message as a corrupt file. */
function describeOverflow(data: Record<string, unknown>): string | undefined {
	const jobs = isObject(data.jobs) ? Object.values(data.jobs) : []
	if (jobs.length > MAX_RECORDED_JOBS) {
		return `This recording holds ${jobs.length} jobs, more than the ${MAX_RECORDED_JOBS} this player can replay.`
	}
	if (Array.isArray(data.timeline) && data.timeline.length > MAX_TIMELINE_FRAMES) {
		return `This recording holds ${data.timeline.length} timeline frames, more than the ${MAX_TIMELINE_FRAMES} this player can animate.`
	}
	// The render budget is the cap a legitimate capture is most likely to trip (the
	// recorders stringify job results verbatim), so name the value that blew it and
	// what about it was too big instead of reporting a format error.
	const samples = isObject(data.assetSamples) ? Object.values(data.assetSamples) : []
	for (const [label, value] of [
		['a recorded job', jobs.find((j) => !withinRenderBudget(j))],
		['the recorded job', withinRenderBudget(data.job) ? undefined : data.job],
		['this flow definition', withinRenderBudget(data.flow) ? undefined : data.flow],
		["this script's inputs", withinRenderBudget(data.schema) ? undefined : data.schema],
		['a recorded asset sample', samples.find((s) => !withinRenderBudget(s))]
	] as const) {
		const over = value === undefined ? undefined : describeValueOverflow(value)
		if (over) return `Cannot replay: ${label} carries ${over}.`
	}
	// Checked at the root of the walk by `isScriptRecording`, where there is no
	// enclosing key for the walk itself to report on.
	for (const [label, size] of [
		["this script's arguments", rowCount(data.args)],
		[
			"this script's schema",
			rowCount((data.schema as Record<string, unknown> | undefined)?.properties)
		]
	] as const) {
		if (size > MAX_MAP_ROWS) {
			return `Cannot replay: ${label} holds ${size} entries, more than the ${MAX_MAP_ROWS} this player renders.`
		}
	}
	return undefined
}

/** A v1 stream's incremental logs may be longer than what its completed job
 * carried (sub-jobs were often backfilled without logs), but the upgraded job
 * is one rendered value, so the reassembly is held under the text budget. */
const MAX_UPGRADED_LOG_CHARS = 2 * 1024 * 1024

/** v1 flow/script/pipeline recordings stored captured live event streams; each
 * stream carried the completed job in its final `completed` event, so they
 * upgrade to the v2 run-based shape by collapsing every stream to that job —
 * with the stream's incremental `new_logs` chunks reassembled, since the
 * completed job was often fetched without them. Recordings already published
 * (the hub) keep replaying this way. Returns undefined when no completed job
 * can be extracted. */
function upgradeV1JobRecording(data: Record<string, unknown>): Record<string, unknown> | undefined {
	const completedJobOf = (stream: unknown): Record<string, unknown> | undefined => {
		if (!isObject(stream)) return undefined
		let job = isObject(stream.initial_job) ? stream.initial_job : undefined
		let streamedLogs = ''
		for (const e of Array.isArray(stream.events) ? stream.events : []) {
			if (!isObject(e) || !isObject(e.data)) continue
			if (typeof e.data.new_logs === 'string') streamedLogs += e.data.new_logs
			if (e.data.completed && isObject(e.data.job)) {
				job = e.data.job as Record<string, unknown>
			}
		}
		if (!job || typeof job.id !== 'string') return undefined
		const jobLogs = typeof job.logs === 'string' ? job.logs : ''
		if (streamedLogs.length > jobLogs.length) {
			job = {
				...job,
				logs:
					streamedLogs.length > MAX_UPGRADED_LOG_CHARS
						? '[logs truncated for recording]\n…' + streamedLogs.slice(-MAX_UPGRADED_LOG_CHARS)
						: streamedLogs
			}
		}
		return job
	}
	const type = data.type === undefined ? 'flow' : data.type
	if (type === 'script') {
		const job = completedJobOf(data.job)
		return job ? { ...data, version: 2, job } : undefined
	}
	if (type !== 'flow' && type !== 'pipeline') return undefined
	if (!isObject(data.jobs)) return undefined
	const jobs: Record<string, Record<string, unknown>> = {}
	for (const stream of Object.values(data.jobs)) {
		const job = completedJobOf(stream)
		if (job) jobs[job.id as string] = job
	}
	if (type === 'pipeline') return { ...data, version: 2, jobs }
	// A flow additionally needs its root: the parentless flow job, mirroring how
	// the v1 player located it.
	const all = Object.values(jobs)
	const rootJob =
		all.find((j) => (j.job_kind === 'flow' || j.job_kind === 'flowpreview') && !j.parent_job) ??
		all.find((j) => !j.parent_job) ??
		all[0]
	if (!rootJob) return undefined
	return { ...data, version: 2, type: 'flow', root_job_id: rootJob.id, jobs }
}

/** Classify a parsed recording and validate it against the player that would
 * mount it. The `type` discriminator picks the validator, so a malformed payload
 * reports the kind it claimed to be instead of falling through to `flow`. */
export function parseRecording(
	input: unknown
): { ok: true; loaded: LoadedRecording } | { ok: false; error: string } {
	if (!isObject(input) || typeof input.version !== 'number') {
		return { ok: false, error: 'This file is not a Windmill recording.' }
	}
	// Before anything looks at what the fields mean: no recording, whatever it holds,
	// may carry more structure than a tab can render. This is what makes the bound
	// exhaustive rather than a list of the fields someone remembered.
	if (countRecordingNodes(input) > MAX_RECORDING_NODES) {
		return {
			ok: false,
			error: `This recording carries more than ${MAX_RECORDING_NODES} values, more than this player can render.`
		}
	}
	let data = input
	// App recordings never changed format and stay at version 1.
	const expectedVersion = data.type === 'app' ? 1 : 2
	if (data.version === 1 && expectedVersion === 2) {
		const upgraded = upgradeV1JobRecording(data)
		if (!upgraded) {
			return {
				ok: false,
				error:
					'This recording was made by an older version of Windmill and can no longer be replayed — re-record it.'
			}
		}
		data = upgraded
	} else if (data.version !== expectedVersion) {
		return {
			ok: false,
			error: 'This recording needs a newer version of Windmill to replay.'
		}
	}
	const type = data.type === undefined ? 'flow' : data.type
	const invalid = (kind: string) => ({
		ok: false as const,
		error: describeOverflow(data) ?? `Invalid ${kind} recording format.`
	})
	switch (type) {
		case 'app':
			return isAppRecording(data)
				? { ok: true, loaded: { kind: 'app', recording: data } }
				: invalid('app')
		case 'script':
			return isScriptRecording(data)
				? { ok: true, loaded: { kind: 'script', recording: data } }
				: invalid('script')
		case 'pipeline':
			return isPipelineRecording(data)
				? { ok: true, loaded: { kind: 'pipeline', recording: data } }
				: invalid('pipeline')
		case 'flow':
			return isFlowRecording(data)
				? { ok: true, loaded: { kind: 'flow', recording: data } }
				: invalid('flow')
		default: {
			// `type` is caller-controlled and only structurally bounded: a payload can
			// carry megabytes in it and reach the page as text. Name it only when it is
			// short enough to be a kind rather than a payload.
			const named = typeof type === 'string' && type.length <= 32 ? ` (${type})` : ''
			return {
				ok: false,
				error: `This recording is of an unknown kind${named} — it may need a newer Windmill.`
			}
		}
	}
}

/** Fetch a recording from `url`, enforcing the download cap while streaming. */
export async function fetchRecording(
	url: string,
	onProgress?: (loaded: number, total: number) => void
): Promise<unknown> {
	const res = await fetch(url)
	if (!res.ok) throw new Error(`HTTP ${res.status} ${res.statusText}`)
	const total = Number(res.headers.get('content-length')) || 0
	if (total > MAX_RECORDING_BYTES) throw new Error(`Recording is too large (${total} bytes)`)
	const reader = res.body?.getReader()
	if (!reader) {
		const text = await res.text()
		if (text.length > MAX_RECORDING_BYTES) throw new Error('Recording exceeded the size limit')
		return JSON.parse(text)
	}
	const chunks: Uint8Array[] = []
	let loaded = 0
	for (;;) {
		const { done, value } = await reader.read()
		if (done) break
		if (!value) continue
		chunks.push(value)
		loaded += value.length
		if (loaded > MAX_RECORDING_BYTES) {
			await reader.cancel()
			throw new Error('Recording exceeded the size limit')
		}
		onProgress?.(loaded, total)
	}
	return JSON.parse(await new Blob(chunks as BlobPart[]).text())
}
