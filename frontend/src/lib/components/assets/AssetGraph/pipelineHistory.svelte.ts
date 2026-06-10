import { JobService } from '$lib/gen'
import type { PipelineEvent } from './activeRunnables.svelte'

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
	let loading = $state(false)
	let truncated = $state(false)
	let error = $state<string | undefined>(undefined)
	// Generation counter: a folder/days change mid-flight must not write a
	// stale response into the new scope (mirrors the page's bodyFetchGen).
	let gen = 0

	async function load(ws: string, prefix: string, days: number) {
		const myGen = ++gen
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
				if (gen !== myGen) return
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
			events = out
			truncated = sawFullPage
		} catch (e: any) {
			if (gen !== myGen) return
			error = e?.body ?? e?.message ?? String(e)
		} finally {
			if (gen === myGen) loading = false
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
		}
	}
}
