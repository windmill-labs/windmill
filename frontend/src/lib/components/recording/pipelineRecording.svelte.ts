import type { Job } from '$lib/gen'
import type { AssetGraphResponse } from '$lib/components/assets/AssetGraph/types'
import { truncateUuids } from './flowRecording.svelte'
import type {
	ActiveReplayData,
	PipelineAssetSample,
	PipelineRecordedCode,
	PipelineRecording,
	PipelineTimelineFrame,
	RecordedJob,
	RecordedNodeState
} from './types'

/**
 * Recorder for a data-pipeline cascade run. Unlike the flow/script recorders
 * there is no single root job streaming sub-jobs over SSE — a pipeline run is a
 * cascade of independent script jobs launched client-side and polled to
 * completion. So this store captures two things:
 *
 *  1. the resolved asset graph (rendered read-only by the player), and
 *  2. a timeline of per-node status snapshots (from the cascade orchestrator's
 *     `onUpdate`), each node mapped to its job id.
 *
 * For each launched node it opens the job's own SSE stream (`watchJob`) to
 * capture incremental logs/result, storing them in the shared `RecordedJob`
 * shape so the player can replay each node's details through the same
 * `JobLoader` replay path the flow/script players use.
 */
export function createPipelineRecording(): PipelineRecordingStore {
	let active = $state(false)
	let startTime = 0
	let folder = ''
	let graph: AssetGraphResponse | undefined = undefined
	let timeline: PipelineTimelineFrame[] = []
	let jobs: Record<string, RecordedJob> = {}
	let assetSamples: Record<string, PipelineAssetSample> = {}
	let codes: Record<string, PipelineRecordedCode> = {}
	let watchedJobs = new Set<string>()
	let jobSources: EventSource[] = []

	function closeSources() {
		jobSources.forEach((es) => es.close())
		jobSources = []
		watchedJobs.clear()
	}

	return {
		get active() {
			return active
		},
		start(f: string, g: AssetGraphResponse) {
			closeSources()
			active = true
			startTime = Date.now()
			folder = f
			// JSON round-trip to strip reactive proxies / non-serializable props.
			graph = JSON.parse(JSON.stringify(g)) as AssetGraphResponse
			timeline = []
			jobs = {}
			assetSamples = {}
			codes = {}
		},
		/** Push a cascade status snapshot. Deep-cloned so a later mutation of the
		 * orchestrator's map can't rewrite an already-captured frame. */
		recordStatuses(statuses: Map<string, RecordedNodeState>) {
			if (!active) return
			const snapshot: Record<string, RecordedNodeState> = {}
			for (const [path, st] of statuses) {
				snapshot[path] = { status: st.status, jobId: st.jobId, error: st.error }
			}
			timeline.push({ t: Date.now() - startTime, statuses: snapshot })
		},
		/** Watch a launched node's SSE stream to capture its incremental
		 * logs/result. Mirrors flowRecording.watchSubJob's log-offset dedup. */
		watchJob(jobId: string, workspace: string) {
			if (!active || watchedJobs.has(jobId)) return
			watchedJobs.add(jobId)

			let logOffset = 0
			const params = new URLSearchParams({
				log_offset: '0',
				running: 'true',
				fast: 'true'
			})
			const url = `/api/w/${workspace}/jobs_u/getupdate_sse/${jobId}?${params}`
			const es = new EventSource(url)
			jobSources.push(es)

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
					if (data.new_logs != null && data.log_offset != null) {
						if (logOffset > 0 && data.log_offset <= logOffset) {
							delete data.new_logs
							delete data.log_offset
						} else {
							logOffset = data.log_offset
						}
					} else if (data.log_offset != null && data.log_offset > logOffset) {
						logOffset = data.log_offset
					}

					if (!jobs[jobId]) {
						jobs[jobId] = {
							initial_job: data.job ? (data.job as Job) : ({ id: jobId } as Job),
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
		/** Fill in a node's completed job (fallback for anything the SSE stream
		 * missed — e.g. a job that finished before its stream was opened).
		 * Callable after stop() so late-fetched completed jobs still attach. */
		addCompletedJob(jobId: string, completedJob: Job) {
			const snapshotJob = $state.snapshot(completedJob) as Job
			if (!jobs[jobId]) {
				jobs[jobId] = { initial_job: snapshotJob, events: [] }
			} else if (!jobs[jobId].initial_job?.id) {
				jobs[jobId].initial_job = snapshotJob
			}
			const hasCompleted = jobs[jobId].events.some((e) => e.data.completed)
			if (!hasCompleted) {
				jobs[jobId].events.push({
					t: Date.now() - startTime,
					data: { completed: true, job: snapshotJob }
				})
			}
		},
		/** Attach a captured asset data-sample (called during finalize, after
		 * the run, for each ducklake/datatable asset). Keyed by `${kind}:${path}`.
		 * Callable after stop() so late captures still attach to the returned
		 * recording (which references the same `assetSamples` object). */
		recordAssetSample(sample: PipelineAssetSample) {
			assetSamples[`${sample.kind}:${sample.path}`] = sample
		},
		/** Attach a runnable's source (called during finalize, per script path).
		 * Callable after stop() so late captures still attach to the returned
		 * recording (which references the same `codes` object). */
		recordCode(path: string, code: PipelineRecordedCode) {
			codes[path] = code
		},
		stop(): PipelineRecording {
			active = false
			closeSources()
			return {
				version: 1,
				type: 'pipeline',
				recorded_at: new Date().toISOString(),
				folder,
				total_duration_ms: Date.now() - startTime,
				graph: graph ?? ({ assets: [], runnables: [], edges: [], triggers: [] } as any),
				timeline,
				jobs,
				assetSamples,
				codes
			}
		},
		/** Convert to the ActiveReplayData shape JobLoader replay consumes. */
		toReplayData(recording: PipelineRecording): ActiveReplayData {
			return { jobs: recording.jobs }
		},
		download(recording: PipelineRecording) {
			const blob = new Blob([truncateUuids(JSON.stringify(recording, null, 2))], {
				type: 'application/json'
			})
			const url = URL.createObjectURL(blob)
			const a = document.createElement('a')
			a.href = url
			a.download = `pipeline-recording-${(recording.folder || 'untitled').replace(/\//g, '-')}-${Date.now()}.json`
			a.click()
			URL.revokeObjectURL(url)
		}
	}
}

export type PipelineRecordingStore = {
	readonly active: boolean
	start(folder: string, graph: AssetGraphResponse): void
	recordStatuses(statuses: Map<string, RecordedNodeState>): void
	watchJob(jobId: string, workspace: string): void
	addCompletedJob(jobId: string, completedJob: Job): void
	recordAssetSample(sample: PipelineAssetSample): void
	recordCode(path: string, code: PipelineRecordedCode): void
	stop(): PipelineRecording
	toReplayData(recording: PipelineRecording): ActiveReplayData
	download(recording: PipelineRecording): void
}
