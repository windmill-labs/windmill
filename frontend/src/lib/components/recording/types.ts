import type { Job, OpenFlow } from '$lib/gen'
import type { AssetGraphResponse } from '$lib/components/assets/AssetGraph/types'

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
