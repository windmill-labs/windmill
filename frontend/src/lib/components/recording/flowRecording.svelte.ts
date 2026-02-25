import type { Job, OpenFlow } from '$lib/gen'
import type { FlowRecording, RecordedJob } from './types'

// Module-level active instances (bypasses context/portal issues)
let activeRecording: FlowRecordingStore | undefined = undefined
let activeReplay: FlowRecording | undefined = undefined
let replayStartTime: number = 0

export function getActiveRecording() {
	return activeRecording
}
export function setActiveRecording(r: FlowRecordingStore | undefined) {
	activeRecording = r
}
export function getActiveReplay() {
	return activeReplay
}
export function setActiveReplay(r: FlowRecording | undefined) {
	activeReplay = r
	replayStartTime = r ? Date.now() : 0
}
export function getReplayStartTime() {
	return replayStartTime
}

export function createFlowRecording() {
	let active = $state(false)
	let startTime = 0
	let flowPath = ''
	let jobs: Record<string, RecordedJob> = {}
	let flow: OpenFlow | undefined = undefined
	let watchedSubJobs = new Set<string>()
	let subJobSources: EventSource[] = []

	return {
		get active() {
			return active
		},
		start(path: string) {
			subJobSources.forEach((es) => es.close())
			subJobSources = []
			watchedSubJobs.clear()
			active = true
			startTime = Date.now()
			flowPath = path
			jobs = {}
			flow = undefined
		},
		setFlow(f: OpenFlow) {
			// JSON round-trip to strip non-serializable properties (event handlers, etc.)
			flow = JSON.parse(JSON.stringify(f)) as OpenFlow
		},
		recordInitialJob(jobId: string, job: Job) {
			if (!active) return
			// $state.snapshot unwraps Svelte 5 reactive proxies (structuredClone fails on them)
			jobs[jobId] = {
				initial_job: $state.snapshot(job) as Job,
				events: []
			}
		},
		recordEvent(jobId: string, data: Record<string, any>) {
			if (!active) return
			if (!jobs[jobId]) {
				jobs[jobId] = {
					initial_job: (data as any).job
						? ($state.snapshot((data as any).job) as Job)
						: ({} as Job),
					events: []
				}
			}
			jobs[jobId].events.push({
				t: Date.now() - startTime,
				data: $state.snapshot(data) as Record<string, any>
			})
		},
		stop(): FlowRecording {
			active = false
			subJobSources.forEach((es) => es.close())
			subJobSources = []
			const recording: FlowRecording = {
				version: 1,
				recorded_at: new Date().toISOString(),
				flow_path: flowPath,
				total_duration_ms: Date.now() - startTime,
				jobs,
				flow
			}
			return recording
		},
		/** Add a completed sub-job to the recording (called after stop for sub-jobs fetched post-completion) */
		addCompletedJob(jobId: string, completedJob: Job) {
			const snapshotJob = $state.snapshot(completedJob) as Job
			if (!jobs[jobId]) {
				jobs[jobId] = {
					initial_job: snapshotJob,
					events: []
				}
			} else if (!jobs[jobId].initial_job?.id) {
				// Replace placeholder initial_job created by recordEvent
				jobs[jobId].initial_job = snapshotJob
			}
			// Only add synthetic completed event if SSE didn't already capture one
			const hasCompleted = jobs[jobId].events.some((e) => e.data.completed)
			if (!hasCompleted) {
				jobs[jobId].events.push({
					t: Date.now() - startTime,
					data: { completed: true, job: snapshotJob }
				})
			}
		},
		/** Watch a sub-job's SSE stream during recording to capture incremental log events */
		watchSubJob(jobId: string, workspace: string) {
			if (!active || watchedSubJobs.has(jobId)) return
			watchedSubJobs.add(jobId)

			let subJobLogOffset = 0

			const params = new URLSearchParams({
				log_offset: '0',
				running: 'true',
				fast: 'true'
			})
			const url = `/api/w/${workspace}/jobs_u/getupdate_sse/${jobId}?${params}`
			const es = new EventSource(url)
			subJobSources.push(es)

			es.onmessage = (event) => {
				try {
					const data = JSON.parse(event.data)
					if (data.type === 'ping' || data.type === 'timeout') return
					if (data.type === 'error' || data.type === 'not_found') {
						es.close()
						return
					}
					if (!active) {
						es.close()
						return
					}

					// Deduplicate log data: SSE may resend full log dumps on reconnect.
					// Drop log data if the offset hasn't advanced past what we already recorded.
					if (data.new_logs != null && data.log_offset != null) {
						if (subJobLogOffset > 0 && data.log_offset <= subJobLogOffset) {
							delete data.new_logs
							delete data.log_offset
						} else {
							subJobLogOffset = data.log_offset
						}
					} else if (data.log_offset != null && data.log_offset > subJobLogOffset) {
						subJobLogOffset = data.log_offset
					}

					// Record initial job from the first event that carries job data
					if (!jobs[jobId] && data.job) {
						jobs[jobId] = {
							initial_job: data.job as Job,
							events: []
						}
					} else if (!jobs[jobId]) {
						jobs[jobId] = {
							initial_job: {} as Job,
							events: []
						}
					}
					jobs[jobId].events.push({
						t: Date.now() - startTime,
						data
					})
					if (data.completed) {
						es.close()
					}
				} catch {
					// Ignore parse errors
				}
			}
			es.onerror = () => {
				es.close()
			}
		},
		download(recording: FlowRecording) {
			const blob = new Blob([JSON.stringify(recording, null, 2)], { type: 'application/json' })
			const url = URL.createObjectURL(blob)
			const a = document.createElement('a')
			a.href = url
			a.download = `flow-recording-${recording.flow_path.replace(/\//g, '-')}-${Date.now()}.json`
			a.click()
			URL.revokeObjectURL(url)
		}
	}
}

export type FlowRecordingStore = ReturnType<typeof createFlowRecording>
