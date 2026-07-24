import type { AssetKind, Job, OpenFlow } from '$lib/gen'
import type { AssetGraphResponse } from '$lib/components/assets/AssetGraph/types'
import type { RawAppInteractionKind } from './rawAppSnapshot'

export type RecordedEvent = {
	t: number
	data: Record<string, any>
}

export type RecordedJob = {
	initial_job: Job
	events: RecordedEvent[]
}

export type FlowRecording = {
	version: 1
	type?: 'flow'
	recorded_at: string
	flow_path: string
	total_duration_ms: number
	jobs: Record<string, RecordedJob>
	flow?: OpenFlow
}

export type ScriptRecording = {
	version: 1
	type: 'script'
	recorded_at: string
	script_path: string
	total_duration_ms: number
	code: string
	language: string
	args: Record<string, any>
	schema?: Record<string, any>
	job: RecordedJob
}

/** Per-node status inside a recorded cascade frame (mirror of
 * cascadeOrchestrator.CascadeNodeState, kept structurally independent so the
 * recording module doesn't depend on the orchestrator internals). */
export type RecordedNodeState = {
	status: 'pending' | 'running' | 'success' | 'failure' | 'skipped'
	jobId?: string
	error?: string
}

/** One frame of the cascade timeline: the full per-path status snapshot at
 * `t` ms since the run started. Replaying these in order reproduces the graph
 * animation (nodes lighting up / turning green/red) a live run would show. */
export type PipelineTimelineFrame = {
	t: number
	statuses: Record<string, RecordedNodeState>
}

/** A captured data-sample of a pipeline asset (ducklake table / datatable),
 * so the player can show what an asset held after the run — offline, without
 * re-querying the backend. Keyed in `assetSamples` by `${kind}:${path}`. */
export type PipelineAssetSample = {
	kind: AssetKind
	path: string
	/** Full asset URI, e.g. `ducklake://main/orders`. */
	uri: string
	/** Column names (in order) of the sampled table. */
	columns: { field: string; datatype?: string }[]
	/** Sampled rows (capped), each a record keyed by column field. */
	rows: unknown[]
	/** Total row count if it could be fetched. */
	rowCount?: number
	/** Set when the sample couldn't be captured (table missing, unsupported…). */
	error?: string
}

export type PipelineRecording = {
	version: 1
	type: 'pipeline'
	recorded_at: string
	folder: string
	total_duration_ms: number
	/** The resolved asset graph rendered read-only by the player. */
	graph: AssetGraphResponse
	/** Ordered cascade status snapshots driving the node animation. */
	timeline: PipelineTimelineFrame[]
	/** Per-node job streams (initial job + SSE events), keyed by job id, so the
	 * player can replay each node's logs/result/args offline via JobLoader. */
	jobs: Record<string, RecordedJob>
	/** Per-asset data samples captured after the run, keyed by `${kind}:${path}`,
	 * so asset nodes are inspectable offline in the player. */
	assetSamples?: Record<string, PipelineAssetSample>
	/** Source code of each runnable, keyed by script path, captured at record
	 * time so the player can show a step's code offline. Absent for recordings
	 * taken before code capture existed (the player degrades gracefully). */
	codes?: Record<string, PipelineRecordedCode>
}

/** A pipeline step's captured source. */
export type PipelineRecordedCode = {
	content: string
	language: string
}

/** One user interaction in a raw-app session recording. `before`/`after` index
 * into {@link RawAppRecording.frames}: the DOM the user acted on, and the DOM
 * once the app settled. Either may be absent when the snapshot budget ran out. */
export type RawAppStep = {
	t: number
	kind: RawAppInteractionKind
	/** One-line description shown in the player, e.g. `Clicked button "Save"`. */
	label: string
	/** The element as a user would name it, e.g. `button "Save"`. */
	target: string
	selector?: string
	value?: string
	before?: number
	after?: number
}

export type RawAppRecording = {
	version: 1
	type: 'app'
	recorded_at: string
	app_path: string
	workspace?: string
	total_duration_ms: number
	/** Size of the app viewport at record time, replayed at the same scale. */
	viewport: { width: number; height: number }
	/** Deduplicated, self-contained DOM snapshots referenced by the steps. */
	frames: string[]
	steps: RawAppStep[]
	/** Set when the snapshot budget was hit and later frames were dropped. */
	truncated?: boolean
}

/** Minimal interface that both flow and script recording stores implement */
export interface ActiveRecording {
	recordInitialJob(jobId: string, job: Job): void
	recordEvent(jobId: string, data: Record<string, any>): void
}

/** Shape needed by JobLoader replay — a map of job ID to recorded data */
export interface ActiveReplayData {
	jobs: Record<string, RecordedJob>
}
