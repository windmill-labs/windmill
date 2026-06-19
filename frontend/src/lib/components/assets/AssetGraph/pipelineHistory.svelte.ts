import { JobService } from '$lib/gen'
import type { PipelineEvent } from './activeRunnables.svelte'

// One asset-cascade dispatch record (from the dispatch_event table). A
// `dispatched` row carries the resolved `child_job_id` (a producer→child job
// edge); a `join_pending` row is a pre-completion AND-join input with no child
// yet — the panel links it to the eventual child of the same subscriber so a
// join's separate trigger chains land in one group.
export type DispatchEdge = {
	producer_job_id: string
	child_job_id?: string
	subscriber_path: string
	outcome: 'dispatched' | 'join_pending'
	asset_path: string
	asset_kind: string
	created_at: string
}

// Page size is the API max; 3 pages keeps the preload bounded on noisy
// folders — the panel notes the truncation instead of silently capping.
const PER_PAGE = 100
const MAX_PAGES = 3

/**
 * One-shot preload of completed folder jobs for the view/preview activity
 * panel — the "last N days" history that the live poll
 * (`useActiveRunnableIds`) intentionally excludes via its session baseline.
 * Kept separate from the poll so its session-count / catch-up-pulse
 * semantics stay untouched; the page merges the two event lists, with the
 * live poll winning on id collisions (it carries fresher status).
 */
export function usePipelineHistory(
	getWorkspace: () => string | undefined,
	getPathPrefix: () => string | undefined,
	getDays: () => number,
	getEnabled: () => boolean
) {
	let events = $state<PipelineEvent[]>([])
	let edges = $state<DispatchEdge[]>([])
	let loading = $state(false)
	let truncated = $state(false)
	let error = $state<string | undefined>(undefined)
	// Generation counter: a folder/days change mid-flight must not write a
	// stale response into the new scope (mirrors the page's bodyFetchGen).
	let gen = 0

	async function load(ws: string, prefix: string, days: number) {
		const myGen = ++gen
		// A folder/days change (or unmount) bumps `gen`; once it diverges from
		// this call's `myGen`, every write below this point belongs to a stale
		// scope and must be dropped.
		const isStale = () => gen !== myGen
		loading = true
		error = undefined
		try {
			const cutoff = new Date(Date.now() - days * 24 * 3600 * 1000).toISOString()
			const out: PipelineEvent[] = []
			let pages = 0
			let sawFullPage = true
			while (pages < MAX_PAGES && sawFullPage) {
				pages += 1
				const rows = await JobService.listCompletedJobs({
					workspace: ws,
					scriptPathStart: prefix,
					jobKinds: ['preview', 'script', 'flow', 'flowpreview', 'flowscript'].join(','),
					startedAfter: cutoff,
					orderDesc: true,
					perPage: PER_PAGE,
					page: pages
				})
				if (isStale()) return
				for (const j of rows) {
					out.push({
						id: j.id,
						path: j.script_path ?? '',
						kind: j.job_kind.startsWith('flow') ? 'flow' : 'script',
						status: j.success ? 'success' : 'failure',
						source: j.schedule_path ? 'schedule' : 'run',
						at: j.started_at ?? j.created_at
					})
				}
				sawFullPage = rows.length === PER_PAGE
			}
			// Cascade edges for the same window. Best-effort: a failure here
			// must not blank the activity list, so it only degrades grouping.
			let edgeRows: DispatchEdge[] = []
			try {
				edgeRows = (await JobService.listAssetDispatchEdges({
					workspace: ws,
					pathStart: prefix,
					createdAfter: cutoff
				})) as DispatchEdge[]
			} catch (e) {
				console.warn('failed to load pipeline dispatch edges', e)
			}
			if (isStale()) return
			events = out
			edges = edgeRows
			truncated = sawFullPage
		} catch (e: any) {
			if (isStale()) return
			error = e?.body ?? e?.message ?? String(e)
		} finally {
			if (!isStale()) loading = false
		}
	}

	// Edges-only refetch (one cheap query, no completed-jobs paging). The
	// `dispatch_event` rows that group a cascade are written server-side only
	// once the producer COMPLETES, so a run launched live (manual or scheduled)
	// has no edges in this one-shot preload — its producer and just-dispatched
	// children would show as separate rows. The page calls this when the live
	// poll surfaces new jobs so fresh cascades group with their producer.
	async function loadEdges(ws: string, prefix: string, days: number) {
		// Skip while a full load owns `edges`; that load sets fresher edges and
		// a concurrent write here could clobber it with a slightly older window.
		if (loading) return
		const myGen = gen
		const cutoff = new Date(Date.now() - days * 24 * 3600 * 1000).toISOString()
		try {
			const edgeRows = (await JobService.listAssetDispatchEdges({
				workspace: ws,
				pathStart: prefix,
				createdAfter: cutoff
			})) as DispatchEdge[]
			if (gen !== myGen) return
			edges = edgeRows
		} catch (e) {
			console.warn('failed to refetch pipeline dispatch edges', e)
		}
	}

	$effect(() => {
		const ws = getWorkspace()
		const prefix = getPathPrefix()
		const days = getDays()
		const enabled = getEnabled()
		if (!enabled || !ws || !prefix) return
		void load(ws, prefix, days)
		return () => {
			// Invalidate in-flight fetches when scope changes / unmount.
			gen++
		}
	})

	return {
		/** Completed folder jobs since the cutoff, newest-first. */
		get events() {
			return events
		},
		/** Asset-cascade producer→child edges over the same window. */
		get edges() {
			return edges
		},
		get loading() {
			return loading
		},
		/** True when the preload hit its page cap before the cutoff. */
		get truncated() {
			return truncated
		},
		get error() {
			return error
		},
		refetch() {
			const ws = getWorkspace()
			const prefix = getPathPrefix()
			if (!ws || !prefix || !getEnabled()) return
			void load(ws, prefix, getDays())
		},
		/** Re-pull just the dispatch edges (see `loadEdges`). */
		refetchEdges() {
			const ws = getWorkspace()
			const prefix = getPathPrefix()
			if (!ws || !prefix || !getEnabled()) return
			void loadEdges(ws, prefix, getDays())
		}
	}
}
