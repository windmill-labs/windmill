// Reusable cascade-execution primitives shared by the pipeline route page and
// the local-dev preview (`PipelineDevView`). The backend asset-trigger
// dispatcher only resolves DEPLOYED rows, so whenever a run involves local /
// draft content the client must orchestrate the closure itself: topological
// order over the graph the user is looking at, each node launched with
// `_wmill_skip_asset_dispatch` so the backend never double-fires the deployed
// part of a mixed chain.

import { JobService, type Preview } from '$lib/gen'
import {
	runCascade,
	runSelection,
	type CascadeRunResult,
	type CascadeNodeState
} from './cascadeOrchestrator'
import { computeDownstreamClosure, computeInducedSchedule } from './graphTraversal'
import { buildLineageDownstreamMap } from './boundedCascade'
import type { AssetGraphResponse } from './types'

export const CASCADE_POLL_INTERVAL_MS = 1000
export const CASCADE_JOB_TIMEOUT_MS = 30 * 60 * 1000

// Data-asset kinds a pipeline graph resolves — the `asset_kinds` filter for the
// `/assets/graph` fetch. Shared so the pipeline editor and deploy-to-hub request
// the same nodes/edges (and can't silently diverge when a kind is added).
export const DATA_ASSET_KINDS = ['s3object', 'ducklake', 'datatable', 'volume']

export type LocalScriptContent = {
	content: string
	language: Preview['language']
	// `// tag <worker-tag>` — routes the preview to that worker (deployed parity).
	tag?: string
}

// Poll a launched cascade job to a terminal state. Capped so a never-terminating
// job can't pin a run guard forever; on timeout it throws, surfaced as a chain
// failure by the orchestrator.
export function makeWaitJobTerminal(
	workspace: string
): (jobId: string) => Promise<'success' | 'failure'> {
	return async function waitJobTerminal(jobId: string): Promise<'success' | 'failure'> {
		const deadline = Date.now() + CASCADE_JOB_TIMEOUT_MS
		while (Date.now() < deadline) {
			try {
				const r = await JobService.getCompletedJobResultMaybe({
					workspace,
					id: jobId,
					getStarted: false
				})
				if (r.completed) return r.success ? 'success' : 'failure'
			} catch {
				// transient — retry on the next tick
			}
			await new Promise((res) => setTimeout(res, CASCADE_POLL_INTERVAL_MS))
		}
		throw new Error(
			`Timed out after ${Math.round(CASCADE_JOB_TIMEOUT_MS / 60000)}min waiting for job ${jobId} to finish`
		)
	}
}

// Build a per-script launch function. When `resolveLocal(path)` yields content,
// the script runs as a preview of that local content (no deploy); otherwise it
// runs the deployed version by path. Always passes `_wmill_skip_asset_dispatch`.
export function makeLaunch(opts: {
	workspace: string
	resolveLocal?: (path: string) => LocalScriptContent | undefined
	tempScriptRefs?: Record<string, string>
	// Extra run args for a specific node (e.g. the uploaded S3Object bound to a
	// `data_upload` cascade root). Merged over `_wmill_skip_asset_dispatch`; all
	// other nodes run with empty inputs as before.
	argsFor?: (path: string) => Record<string, any> | undefined
	onLaunched?: (path: string, jobId: string) => void
}): (path: string) => Promise<string> {
	return async function launch(path: string): Promise<string> {
		const local = opts.resolveLocal?.(path)
		// Caller args (e.g. the run form for the cascade root) must NOT be able to
		// re-enable backend asset dispatch while the client orchestrates the closure
		// — that would double-run downstream / run deployed subscribers. Drop any
		// `_wmill_skip_asset_dispatch` a caller supplied, and always spread it LAST.
		const { _wmill_skip_asset_dispatch: _reserved, ...extra } = opts.argsFor?.(path) ?? {}
		void _reserved
		let jobId: string
		if (local) {
			if (!local.content || !local.language) {
				throw new Error(`local script ${path} has no content/language`)
			}
			jobId = await JobService.runScriptPreview({
				workspace: opts.workspace,
				requestBody: {
					content: local.content,
					language: local.language,
					path,
					args: { ...extra, _wmill_skip_asset_dispatch: true },
					...(local.tag ? { tag: local.tag } : {}),
					...(opts.tempScriptRefs ? { temp_script_refs: opts.tempScriptRefs } : {})
				}
			})
		} else {
			jobId = await JobService.runScriptByPath({
				workspace: opts.workspace,
				path,
				requestBody: { ...extra, _wmill_skip_asset_dispatch: true }
			})
		}
		opts.onLaunched?.(path, jobId)
		return jobId
	}
}

// Run `root` plus its full downstream closure over the given graph.
export async function runDownstreamCascade(opts: {
	graph: AssetGraphResponse
	root: string
	launch: (path: string) => Promise<string>
	waitTerminal: (jobId: string) => Promise<'success' | 'failure'>
	onUpdate?: (statuses: Map<string, CascadeNodeState>) => void
}): Promise<CascadeRunResult & { cyclic: string[] }> {
	const closure = computeDownstreamClosure(opts.graph, opts.root)
	const res = await runCascade({
		closure,
		root: opts.root,
		launch: opts.launch,
		waitTerminal: opts.waitTerminal,
		onUpdate: opts.onUpdate
	})
	return { ...res, cyclic: closure.cyclic }
}

// Run a bounded selection of scripts (the induced schedule over the lineage DAG).
export async function runBoundedCascade(opts: {
	graph: AssetGraphResponse
	scripts: Set<string>
	launch: (path: string) => Promise<string>
	waitTerminal: (jobId: string) => Promise<'success' | 'failure'>
	onUpdate?: (statuses: Map<string, CascadeNodeState>) => void
}): Promise<CascadeRunResult & { cyclic: string[] }> {
	// Read-aware adjacency (NOT the default write-edge map) so a pure-reader
	// member runs after its producer — parity with the route page's bounded run
	// and the CLI `topoOrder`. `cyclic` is surfaced so callers can warn instead of
	// silently dropping scripts stuck on a dependency cycle.
	const schedule = computeInducedSchedule(
		opts.graph,
		opts.scripts,
		buildLineageDownstreamMap(opts.graph)
	)
	const res = await runSelection({
		schedule,
		launch: opts.launch,
		waitTerminal: opts.waitTerminal,
		onUpdate: opts.onUpdate
	})
	return { ...res, cyclic: schedule.cyclic }
}
