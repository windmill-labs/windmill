/**
 * Validation for a recording arriving from outside — an uploaded file or a
 * `?src=` URL. Both replay pages go through this: a recording is caller-supplied
 * data that a player indexes into and renders per step, so its shape, its
 * cardinality and its sizes all have to hold before anything mounts.
 */
import type {
	FlowRecording,
	PipelineRecording,
	RawAppRecording,
	RecordedJob,
	ScriptRecording
} from './types'
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

/**
 * Total structure in the whole recording — the last line, applied before any
 * per-kind validator runs.
 *
 * The per-value budgets below are precise but they only cover values that were
 * *named*, and six review rounds of this file each found a field nobody had named.
 * This one doesn't need to know the field: with `MAX_RECORDING_BYTES` bounding the
 * bytes and this bounding the structure inside them, an unnamed field cannot be
 * unboundedly large even before anyone decides what it renders as. Far above any
 * real capture (the largest recorded here is ~19k nodes) and far below what a tab
 * survives rendering.
 */
export const MAX_RECORDING_NODES = 2_000_000
/* Caps on the job-stream recordings (flow/script/pipeline). Each recorded job
 * mounts a JobLoader and each of its events costs a `setTimeout` created up front,
 * so the counts — not just the byte size — decide whether the tab survives. */
export const MAX_RECORDED_JOBS = 2000
/** `JobLoader.watchJob` schedules every event of a job in one pass, so this is a
 * count of timers created at once; events at `t: 0` all fire in the same frame, each
 * one a reactive update. Generous against reality (a long streaming job records on
 * the order of thousands) and survivable when they all land together. */
export const MAX_EVENTS_PER_JOB = 5000
export const MAX_RECORDED_JOB_EVENTS = 20_000
/**
 * The backstop: how much structure any single recorded value may expand into. One
 * value is what a component renders eagerly (a job state, a flow definition, an
 * asset sample); the *number* of values is bounded by the counts above.
 *
 * This is deliberately generous, because it is not the precise bound — it exists to
 * catch the keys nobody has named yet. Anything that mounts a *component* per entry
 * is far more expensive than a node and gets {@link MAX_COMPONENT_FANOUT} on top.
 */
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
/**
 * Entries in a collection whose renderer mounts a *component* each, rather than a
 * cell or a row: `render_all` (a nested `DisplayResult` per entry, recursively) and
 * `data_tests` (a checklist item per entry, and it renders ahead of every size
 * fallback `DisplayResult` has). Two orders of magnitude below the node budget
 * because that is roughly the cost ratio. Counted through the serialized form too —
 * these arrive as JSON strings often enough that the renderer parses them itself,
 * and a string is one node no matter what it decodes to.
 */
export const MAX_COMPONENT_FANOUT = 1000
/** Every frame reassigns the whole per-node status map on a timer, and each
 * reassignment rebuilds the derived id/state maps over the entire key set — a
 * per-entry cost that recurs, so it keeps a cap tighter than the node budget. */
export const MAX_TIMELINE_FRAMES = 20_000
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
				typeof s.t === 'number' &&
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

/** Entry count of a fan-out collection, decoding the serialized form the renderers
 * accept — a JSON string is one node however many components it expands into. */
function fanoutLength(v: unknown): number {
	if (Array.isArray(v)) return v.length
	if (typeof v === 'string' && v.length <= MAX_VALUE_STRING_CHARS) {
		try {
			const decoded = JSON.parse(v)
			return Array.isArray(decoded) ? decoded.length : 0
		} catch {
			return 0
		}
	}
	return 0
}

/**
 * Walks one recorded value and reports why it is too big to render, or `undefined`.
 *
 * Three different bounds, because they catch three different things and none
 * subsumes the others:
 *  - **structure** (`MAX_VALUE_NODES`) is the backstop, and the only one that covers
 *    keys nobody has named. Widen this rather than adding a named cap;
 *  - **text** (`MAX_VALUE_STRING_CHARS`), because a string is one node however long,
 *    so structure cannot see a 60 MB inline script;
 *  - **component fan-out** (`MAX_COMPONENT_FANOUT`), because mounting a component per
 *    entry costs orders of magnitude more than a node, so a collection well inside
 *    the structural budget can still be fatal.
 *
 * Bails as soon as any bound is blown, so the walk can't itself be the denial of
 * service, and refuses past `MAX_VALUE_DEPTH` rather than overflowing this stack.
 */
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
			// Cumulative across the value, not per array: `render_all` nests, so 300
			// arrays of 300 are 90k components with no single array over the cap.
			if (COMPONENT_FANOUT_KEYS.includes(k)) {
				budget.fanout -= fanoutLength(v[k])
				if (budget.fanout < 0) {
					return `more than ${MAX_COMPONENT_FANOUT} \`${k}\`-style entries, each of which mounts its own component`
				}
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
	const walk = (x: unknown, depth: number) => {
		if (budget.n <= 0 || depth > MAX_VALUE_DEPTH) return
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
	return count
}

/** True when one recorded value is renderable. Apply this to each value a component
 * expands eagerly (a job's args/result/flow_status, a flow definition, an asset
 * sample); the *number* of such values is bounded separately. */
const withinRenderBudget = (v: unknown) => describeValueOverflow(v) === undefined

/** A RecordedJob whose events all carry an object `data`: JobLoader replays each
 * `event.data` in a `setTimeout`, whose throw a Svelte boundary can't catch, so a
 * malformed event has to be rejected at load. Each job state is also held to the
 * render budget, which covers everything hanging off it — `args` (a JobArgs row per
 * key), `result` (`render_all` fan-out, `data_tests` checklists), `flow_status`
 * (a component subtree per module) — at any depth. */
function isRecordedJob(j: unknown): j is RecordedJob {
	return (
		isObject(j) &&
		isObject(j.initial_job) &&
		withinRenderBudget(j.initial_job) &&
		isBoundedArray(j.events, MAX_EVENTS_PER_JOB) &&
		j.events.every((e) => isObject(e) && isObject(e.data) && withinRenderBudget(e.data))
	)
}

/** The `jobs` map every job-stream recording carries, bounded on both the number
 * of streams and the total number of events across them. */
function isJobsMap(v: unknown): v is Record<string, RecordedJob> {
	if (!isObject(v)) return false
	const jobs = Object.values(v)
	if (jobs.length > MAX_RECORDED_JOBS) return false
	let events = 0
	for (const j of jobs) {
		if (!isRecordedJob(j)) return false
		events += j.events.length
		if (events > MAX_RECORDED_JOB_EVENTS) return false
	}
	return true
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
	if (!isObject(data) || data.version !== 1 || data.type !== 'script') return false
	// `code` is highlighted in one pass and `language` selects the grammar.
	return (
		hasValidHeader(data, 'script_path') &&
		isRecordedJob(data.job) &&
		isBoundedCode(data.code) &&
		// Selects a highlighter grammar and is rendered in the player's header.
		isShortText(data.language, true) &&
		// SchemaForm mounts a field per property, recursing into nested objects, and
		// JobArgs a row per arg — so the budget, not a top-level key count.
		withinRenderBudget(data.schema) &&
		withinRenderBudget(data.args)
	)
}

/** True when `data` is a well-formed pipeline recording. */
export function isPipelineRecording(data: unknown): data is PipelineRecording {
	if (!isObject(data) || data.version !== 1 || data.type !== 'pipeline') return false
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
					// An errored sample renders the message instead of the table, so it needs a
					// bound of its own — the table branch's budget never sees it.
					((isShortText(s.error, true) && s.error !== '') ||
						(isObjectArray(s.rows, MAX_SAMPLE_ROWS) &&
							isObjectArray(s.columns, MAX_SAMPLE_COLUMNS) &&
							(s.rows as unknown[]).length * (s.columns as unknown[]).length <= MAX_SAMPLE_CELLS &&
							// The cells' values are formatted individually on top of that.
							withinRenderBudget(s)))
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
	if (!isObject(data) || data.version !== 1) return false
	if (data.type !== undefined && data.type !== 'flow') return false
	if (!hasValidHeader(data, 'flow_path') || !isJobsMap(data.jobs)) return false
	if (data.flow === undefined) return true
	// The player hands the whole `flow` to FlowViewer, so `schema` renders (Input
	// Schema tab, Input node) just like `value` does — budget one level up.
	if (!isObject(data.flow) || !withinRenderBudget(data.flow)) return false
	const value = data.flow.value
	if (value === undefined) return true
	return isObject(value) && countFlowModules(value.modules, MAX_FLOW_MODULES) <= MAX_FLOW_MODULES
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
	const events = jobs.reduce(
		(sum: number, j) => sum + (isObject(j) && Array.isArray(j.events) ? j.events.length : 0),
		0
	)
	if (events > MAX_RECORDED_JOB_EVENTS) {
		return `This recording holds ${events} job events, more than the ${MAX_RECORDED_JOB_EVENTS} this player can replay.`
	}
	if (Array.isArray(data.timeline) && data.timeline.length > MAX_TIMELINE_FRAMES) {
		return `This recording holds ${data.timeline.length} timeline frames, more than the ${MAX_TIMELINE_FRAMES} this player can animate.`
	}
	// The render budget is the cap a legitimate capture is most likely to trip (the
	// recorders stringify job results verbatim), so name the value that blew it and
	// what about it was too big instead of reporting a format error.
	for (const [label, value] of [
		['a recorded job', jobs.find((j) => !withinRenderBudget(j))],
		['this flow definition', withinRenderBudget(data.flow) ? undefined : data.flow],
		["this script's inputs", withinRenderBudget(data.schema) ? undefined : data.schema]
	] as const) {
		const over = value === undefined ? undefined : describeValueOverflow(value)
		if (over) return `Cannot replay: ${label} carries ${over}.`
	}
	return undefined
}

/** Classify a parsed recording and validate it against the player that would
 * mount it. The `type` discriminator picks the validator, so a malformed payload
 * reports the kind it claimed to be instead of falling through to `flow`. */
export function parseRecording(
	data: unknown
): { ok: true; loaded: LoadedRecording } | { ok: false; error: string } {
	if (!isObject(data) || data.version !== 1) {
		return { ok: false, error: 'This file is not a Windmill recording.' }
	}
	// Before anything looks at what the fields mean: no recording, whatever it holds,
	// may carry more structure than a tab can render. This is what makes the bound
	// exhaustive rather than a list of the fields someone remembered.
	if (countRecordingNodes(data) > MAX_RECORDING_NODES) {
		return {
			ok: false,
			error: `This recording carries more than ${MAX_RECORDING_NODES} values, more than this player can render.`
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
		default:
			return {
				ok: false,
				error: `This recording is of an unknown kind (${String(type)}) — it may need a newer Windmill.`
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
