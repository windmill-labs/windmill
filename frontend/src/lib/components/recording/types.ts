import type { Job, OpenFlow } from '$lib/gen'

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
	job: RecordedJob
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
