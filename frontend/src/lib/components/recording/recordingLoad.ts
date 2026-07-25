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

/* Caps on the job-stream recordings (flow/script/pipeline). Each recorded job
 * mounts a JobLoader and each of its events costs a `setTimeout`, so the counts —
 * not just the byte size — decide whether the tab survives the render. Set far
 * above any real capture: a wide for-loop flow records a job per iteration. */
export const MAX_RECORDED_JOBS = 2000
export const MAX_RECORDED_JOB_EVENTS = 200_000
/** A flow job's `flow_status.modules` becomes one component subtree per entry, so
 * a single job can be a render bomb regardless of how few jobs the map holds. It
 * arrives on `initial_job` and on any event carrying a status update. */
export const MAX_FLOW_STATUS_MODULES = 20_000
/** Every frame reassigns the whole per-node status map on a timer, and each
 * reassignment rebuilds the derived id/state maps over the entire key set. */
export const MAX_TIMELINE_FRAMES = 20_000
export const MAX_FRAME_STATUSES = 5000
/** Graph elements each become a rendered canvas node or edge. */
export const MAX_GRAPH_ELEMENTS = 2000
/* An asset sample renders as a `rows × columns` table of plain `<td>`s, so the
 * product is what costs, and the per-axis caps alone would allow millions of
 * cells from a tiny payload of empty row objects. */
export const MAX_SAMPLE_ROWS = 5000
export const MAX_SAMPLE_COLUMNS = 500
export const MAX_SAMPLE_CELLS = 100_000
/** Captured source is syntax-highlighted in one pass. */
export const MAX_CODE_CHARS = 4 * 1024 * 1024
/** `SchemaForm` and `JobArgs` render a row per property of a script recording's
 * `args`/`schema.properties`, so those maps are render cardinality too. */
export const MAX_ARG_PROPERTIES = 2000
/** `FlowGraphV2` builds and lays out a node per module, recursing into branches and
 * loops, plus one per note. A flow definition is a nested structure, so the count
 * that matters is the total across the tree, not the top-level array's length. */
export const MAX_FLOW_MODULES = 5000
export const MAX_FLOW_NOTES = 1000

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

/** `flow_status.modules` on anything that carries a flow status, bounded wherever
 * it appears rather than only on `initial_job` — a status update arriving as an
 * event lands in the same renderer. */
function hasBoundedFlowStatus(v: unknown): boolean {
	if (!isObject(v)) return true
	const status = v.flow_status
	if (!isObject(status) || status.modules === undefined) return true
	return isBoundedArray(status.modules, MAX_FLOW_STATUS_MODULES)
}

/** A RecordedJob whose events all carry an object `data`: JobLoader replays each
 * `event.data` in a `setTimeout`, whose throw a Svelte boundary can't catch, so a
 * malformed event has to be rejected at load. */
function isRecordedJob(j: unknown): j is RecordedJob {
	return (
		isObject(j) &&
		isObject(j.initial_job) &&
		hasBoundedFlowStatus(j.initial_job) &&
		isBoundedArray(j.events, MAX_RECORDED_JOB_EVENTS) &&
		j.events.every(
			(e) =>
				isObject(e) &&
				isObject(e.data) &&
				hasBoundedFlowStatus(e.data) &&
				hasBoundedFlowStatus(e.data.job)
		)
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

const isBoundedMap = (v: unknown, max: number) =>
	v === undefined || (isObject(v) && Object.keys(v).length <= max)

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
		typeof data.language === 'string' &&
		(data.schema === undefined ||
			(isObject(data.schema) && isBoundedMap(data.schema.properties, MAX_ARG_PROPERTIES))) &&
		isBoundedMap(data.args, MAX_ARG_PROPERTIES)
	)
}

/** True when `data` is a well-formed pipeline recording. */
export function isPipelineRecording(data: unknown): data is PipelineRecording {
	if (!isObject(data) || data.version !== 1 || data.type !== 'pipeline') return false
	const g = data.graph
	const validGraph =
		isObject(g) &&
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
				Object.values(f.statuses).every(isObject)
		)
	// A sample renders `rows`/`columns` unless it carries a non-empty `error`.
	const validSamples =
		data.assetSamples === undefined ||
		(isObject(data.assetSamples) &&
			Object.values(data.assetSamples).every(
				(s) =>
					isObject(s) &&
					((typeof s.error === 'string' && s.error !== '') ||
						(isObjectArray(s.rows, MAX_SAMPLE_ROWS) &&
							isObjectArray(s.columns, MAX_SAMPLE_COLUMNS) &&
							(s.rows as unknown[]).length * (s.columns as unknown[]).length <= MAX_SAMPLE_CELLS))
			))
	const validCodes =
		data.codes === undefined ||
		(isObject(data.codes) &&
			Object.values(data.codes).every(
				(c) => isObject(c) && isBoundedCode(c.content) && typeof c.language === 'string'
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
	if (!isObject(data.flow)) return false
	const value = data.flow.value
	if (value === undefined) return true
	return (
		isObject(value) &&
		countFlowModules(value.modules, MAX_FLOW_MODULES) <= MAX_FLOW_MODULES &&
		(value.notes === undefined || isBoundedArray(value.notes, MAX_FLOW_NOTES))
	)
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
