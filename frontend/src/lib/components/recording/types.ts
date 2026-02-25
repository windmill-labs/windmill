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
	recorded_at: string
	flow_path: string
	total_duration_ms: number
	jobs: Record<string, RecordedJob>
	flow?: OpenFlow
}
