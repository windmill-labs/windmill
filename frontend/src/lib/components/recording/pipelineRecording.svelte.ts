import { ScriptService, type Job } from '$lib/gen'
import type { AssetGraphResponse } from '$lib/components/assets/AssetGraph/types'
import { runBoundedCascade } from '$lib/components/assets/AssetGraph/cascadeRun'
import type {
	CascadeNodeState,
	CascadeRunResult
} from '$lib/components/assets/AssetGraph/cascadeOrchestrator'
import { downloadRecordingJson, fetchJobWithFullLogs } from './runRecording'
import { capturePipelineAssetSample } from './pipelineAssetSample'
import type {
	PipelineAssetSample,
	PipelineRecordedCode,
	PipelineRecording,
	PipelineTimelineFrame,
	RecordedNodeState
} from './types'

/**
 * Recorder for a data-pipeline cascade run. A pipeline run is a cascade of
 * independent script jobs launched client-side, so unlike the flow/script
 * recordings there is no root job whose status ties the run together — the
 * cascade orchestrator's own status snapshots are that record. The store
 * captures:
 *
 *  1. the resolved asset graph (rendered read-only by the player),
 *  2. a timeline of per-node status snapshots (from the cascade orchestrator's
 *     `onUpdate`), each node mapped to its job id, and
 *  3. each node's completed job, fetched after the run (`finalize`); the
 *     player synthesizes its replay stream from it.
 */
export function createPipelineRecording(): PipelineRecordingStore {
	let active = $state(false)
	let startTime = 0
	let folder = ''
	let graph: AssetGraphResponse | undefined = undefined
	let timeline: PipelineTimelineFrame[] = []
	let jobs: Record<string, Job> = {}
	let assetSamples: Record<string, PipelineAssetSample> = {}
	let codes: Record<string, PipelineRecordedCode> = {}

	return {
		get active() {
			return active
		},
		start(f: string, g: AssetGraphResponse) {
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
		/** Attach a node's completed job. Callable after stop() so the
		 * post-run fetches still attach to the returned recording. */
		setJob(jobId: string, completedJob: Job) {
			jobs[jobId] = $state.snapshot(completedJob) as Job
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
			return {
				version: 2,
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
			downloadRecordingJson(
				recording,
				`pipeline-recording-${(recording.folder || 'untitled').replace(/\//g, '-')}`
			)
		}
	}
}

export type PipelineRecordingStore = {
	readonly active: boolean
	start(folder: string, graph: AssetGraphResponse): void
	recordStatuses(statuses: Map<string, RecordedNodeState>): void
	setJob(jobId: string, completedJob: Job): void
	recordAssetSample(sample: PipelineAssetSample): void
	recordCode(path: string, code: PipelineRecordedCode): void
	stop(): PipelineRecording
	download(recording: PipelineRecording): void
}

// Max asset samples in flight during finalize — each is several preview jobs.
const ASSET_SAMPLE_CONCURRENCY = 4
// Max node-job fetches in flight during finalize (two requests each).
const JOB_FETCH_CONCURRENCY = 20

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
 * Stop the recorder and attach everything the run left behind: each node's
 * completed job (with its logs and result), a data-sample per
 * ducklake/datatable asset (offline table preview), and each step's source (by
 * the exact hash that ran). Every fetch is best-effort — a step we can't
 * resolve just replays with less detail. Shared by the pipeline editor's
 * recorder and deploy-to-hub so both produce identical recordings.
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
	// Each fetch is two requests, and a wide pipeline has a node per script —
	// bound the fan-out like the asset sampling below.
	await forEachWithConcurrency([...jobIds], JOB_FETCH_CONCURRENCY, async (jobId) => {
		try {
			store.setJob(jobId, await fetchJobWithFullLogs(ws, jobId))
		} catch {
			// best-effort — a job we can't fetch just isn't inspectable in the player
		}
	})
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
	for (const j of Object.values(rec.jobs) as {
		job_kind?: string
		script_path?: string
		script_hash?: string
	}[]) {
		if (j?.job_kind === 'script' && j.script_path && j.script_hash) {
			codeByPath.set(j.script_path, j.script_hash)
		}
	}
	await forEachWithConcurrency([...codeByPath], JOB_FETCH_CONCURRENCY, async ([path, hash]) => {
		try {
			const s = await ScriptService.getScriptByHash({ workspace: ws, hash })
			store.recordCode(path, { content: s.content, language: s.language })
		} catch {
			// best-effort — a step we can't fetch just has no code in the player
		}
	})
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
			launch: opts.launch,
			waitTerminal: opts.waitTerminal,
			onUpdate: (statuses) => {
				store.recordStatuses(statuses)
				opts.onUpdate?.(statuses)
			}
		})
	} catch (e) {
		store.stop()
		throw e
	}
	const recording = await finalizePipelineRecording(store, opts.workspace)
	return { recording, result }
}
