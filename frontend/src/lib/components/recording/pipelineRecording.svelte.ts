import { JobService, ScriptService, type Job } from '$lib/gen'
import type { AssetGraphResponse } from '$lib/components/assets/AssetGraph/types'
import { runBoundedCascade } from '$lib/components/assets/AssetGraph/cascadeRun'
import type {
	CascadeNodeState,
	CascadeRunResult
} from '$lib/components/assets/AssetGraph/cascadeOrchestrator'
import { truncateUuids } from './flowRecording.svelte'
import { capturePipelineAssetSample } from './pipelineAssetSample'
import type {
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
	download(recording: PipelineRecording): void
}

// Max asset samples in flight during finalize — each is several preview jobs.
const ASSET_SAMPLE_CONCURRENCY = 4

/** Run `fn` over `items` at most `limit` at a time (sequential batches). */
async function forEachWithConcurrency<T>(
	items: T[],
	limit: number,
	fn: (item: T) => Promise<void>
): Promise<void> {
	for (let i = 0; i < items.length; i += limit) {
		await Promise.all(items.slice(i, i + limit).map(fn))
	}
}

/**
 * Stop the recorder and enrich the recording with data the live SSE streams
 * can't guarantee: each node's completed job (a fast job may finish before its
 * stream opens), a data-sample per ducklake/datatable asset (offline table
 * preview), and each step's source (by the exact hash that ran). Every fetch is
 * best-effort — a step we can't resolve just replays with less detail. Shared by
 * the pipeline editor's recorder and deploy-to-hub so both produce identical
 * recordings.
 */
export async function finalizePipelineRecording(
	store: PipelineRecordingStore,
	workspace: string | undefined
): Promise<PipelineRecording> {
	const rec = store.stop()
	if (!workspace) return rec
	const ws = workspace
	const jobIds = new Set<string>()
	for (const frame of rec.timeline) {
		for (const st of Object.values(frame.statuses)) {
			if (st.jobId) jobIds.add(st.jobId)
		}
	}
	await Promise.all(
		[...jobIds].map(async (jobId) => {
			if (rec.jobs[jobId]?.events.some((e) => e.data.completed)) return
			try {
				const j = await JobService.getJob({ workspace: ws, id: jobId })
				store.addCompletedJob(jobId, j)
			} catch {
				// best-effort — a job we can't fetch just replays from its stream
			}
		})
	)
	// Each asset sample runs a metadata scan + a SELECT + a COUNT preview job, so a
	// wide pipeline could fan out hundreds of jobs at once. Bound the concurrency
	// to keep the recorder from saturating the worker pool.
	const sampleTargets = (rec.graph.assets ?? []).filter(
		(a) => a.kind === 'ducklake' || a.kind === 'datatable'
	)
	await forEachWithConcurrency(sampleTargets, ASSET_SAMPLE_CONCURRENCY, async (a) => {
		const sample = await capturePipelineAssetSample(ws, a.kind, a.path)
		store.recordAssetSample(sample)
	})
	const codeByPath = new Map<string, string>()
	for (const r of Object.values(rec.jobs)) {
		const j = r.events.find((e) => e.data.completed)?.data.job as
			| { job_kind?: string; script_path?: string; script_hash?: string }
			| undefined
		if (j?.job_kind === 'script' && j.script_path && j.script_hash) {
			codeByPath.set(j.script_path, j.script_hash)
		}
	}
	await Promise.all(
		[...codeByPath].map(async ([path, hash]) => {
			try {
				const s = await ScriptService.getScriptByHash({ workspace: ws, hash })
				store.recordCode(path, { content: s.content, language: s.language })
			} catch {
				// best-effort — a step we can't fetch just has no code in the player
			}
		})
	)
	return rec
}

/**
 * Run a folder's pipeline cascade end-to-end and capture it into a
 * PipelineRecording — the self-contained path used by deploy-to-hub, where
 * there is no editor page orchestrating the run. `launch`/`waitTerminal` are
 * supplied by the caller (deployed-only launch, poll-based wait); this wires
 * status/job capture around them and finalizes.
 */
export async function capturePipelineRecording(opts: {
	workspace: string
	folder: string
	graph: AssetGraphResponse
	scriptPaths: Set<string>
	launch: (path: string) => Promise<string>
	waitTerminal: (jobId: string) => Promise<'success' | 'failure'>
	onUpdate?: (statuses: Map<string, CascadeNodeState>) => void
}): Promise<{ recording: PipelineRecording; result: CascadeRunResult & { cyclic: string[] } }> {
	const store = createPipelineRecording()
	store.start(opts.folder, opts.graph)
	let result: CascadeRunResult & { cyclic: string[] }
	try {
		result = await runBoundedCascade({
			graph: opts.graph,
			scripts: opts.scriptPaths,
			launch: async (path) => {
				const jobId = await opts.launch(path)
				// No-op unless the store is active; captures the node's stream.
				store.watchJob(jobId, opts.workspace)
				return jobId
			},
			waitTerminal: opts.waitTerminal,
			onUpdate: (statuses) => {
				store.recordStatuses(statuses)
				opts.onUpdate?.(statuses)
			}
		})
	} catch (e) {
		// The cascade threw before finalize could `stop()` the store: close the
		// per-node SSE streams `watchJob` opened so they don't dangle.
		store.stop()
		throw e
	}
	const recording = await finalizePipelineRecording(store, opts.workspace)
	return { recording, result }
}
