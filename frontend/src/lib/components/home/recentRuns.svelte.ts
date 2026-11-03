import { FlowService, JobService, ScriptService, type ListRecentRunsResponse } from '$lib/gen'
import { getLocalSetting, storeLocalSetting } from '$lib/utils'
import { sendUserToast } from '$lib/toast'
import { IsDocumentVisible, useInterval } from 'runed'
import { persistedResource } from '$lib/persistedResource.svelte'
import { homeComposerContext } from '$lib/components/sessions/homeComposerContext.svelte'
import { runContextElement } from '$lib/components/copilot/chat/context'

export type RecentRuns = ListRecentRunsResponse
export type RecentRunItem = RecentRuns['items'][number]
export type RecentRun = RecentRunItem['runs'][number]
export type RecentRunStatus = RecentRun['status']

/** Home stays open in many tabs and each read can examine tens of thousands of completions, while
 * the panel only shows ages to the minute: a tighter rate multiplies that load for nothing. */
const REFRESH_MS = 30000

/** `listJobs` caps a page at 100; the passes bound one click at the runs the endpoint scans. */
const RESOLVE_PAGE = 100
const MAX_RESOLVE_PASSES = 50
/** The job kinds each item kind is read from (`jobs_recent_runs.rs`). A `singlestepflow` wraps a
 * script or a flow and the job list cannot tell which, so it is left out when a script and a flow
 * share the path: resolving one would otherwise resolve the other's failures. */
async function resolveJobKinds(workspace: string, item: RecentRunItem): Promise<string> {
	if (item.kind === 'agent') return 'flowpreview'
	const shared =
		item.kind === 'script'
			? await FlowService.existsFlowByPath({ workspace, path: item.path })
			: await ScriptService.existsScriptByPath({ workspace, path: item.path })
	return shared ? item.kind : `${item.kind},singlestepflow`
}

/**
 * What ran recently in the workspace, read from `listRecentRuns`. The side panel and the header
 * status read the same instance: the counts they show must never disagree. Nothing is read while
 * `enabled` is false: each read scans the window's runs.
 */
export function useRecentRuns(
	args: () => { workspace: string | undefined; folder?: string; enabled: boolean }
) {
	// A refresh that fails (a backend restart) throws, which keeps what is on screen rather than
	// surfacing an error every few seconds; a failed first load shows the cached runs, or nothing.
	const runs = persistedResource(
		args,
		async ({ workspace, folder, enabled }) =>
			workspace && enabled ? await JobService.listRecentRuns({ workspace, folder }) : undefined,
		{
			name: 'recent_runs',
			version: 2,
			scope: ({ workspace, folder, enabled }) =>
				workspace && enabled ? { workspace, args: { folder } } : undefined
		}
	)
	// The rows' ages ("3m") are read against this, so they move on with each refresh.
	let now = $state(Date.now())
	const visible = new IsDocumentVisible()
	useInterval(REFRESH_MS, {
		callback: () => {
			if (!visible.current || !args().enabled) return
			now = Date.now()
			if (!runs.loading) runs.refetch()
		}
	})
	return {
		get current() {
			return runs.current
		},
		/** A read is in flight; with no `current` yet, nothing is known about the window. */
		get loading() {
			return runs.loading
		},
		/** The latest read failed: with no `current`, the panel would otherwise read as empty. */
		get failed() {
			return runs.error !== undefined
		},
		retry() {
			if (!runs.loading) runs.refetch()
		},
		get now() {
			return now
		},
		/** Marks every unresolved failure of `item` in the window as resolved. */
		async resolveFailures(item: RecentRunItem) {
			const workspace = args().workspace
			if (!workspace) return
			const windowHours = runs.current?.window_hours ?? 24
			const completedAfter = new Date(Date.now() - windowHours * 3600_000).toISOString()
			let resolved = 0
			try {
				const jobKinds = await resolveJobKinds(workspace, item)
				// The ids are read on click rather than carried by the polled response, which would
				// hold every failure of a constantly failing item. Each pass resolves what it read,
				// so the next first page holds the failures still unresolved.
				for (let pass = 0; pass < MAX_RESOLVE_PASSES; pass++) {
					const failures = await JobService.listJobs({
						workspace,
						scriptPathExact: item.path,
						jobKinds,
						hasNullParent: true,
						status: 'failure',
						resolved: false,
						completedAfter,
						perPage: RESOLVE_PAGE
					})
					if (failures.length === 0) break
					const affected = await JobService.resolveCompletedJobs({
						workspace,
						requestBody: { job_ids: failures.map((j) => j.id) }
					})
					resolved += affected.length
					// A failure the caller may not resolve stays on the first page: reading again would
					// return it forever.
					if (affected.length < failures.length || failures.length < RESOLVE_PAGE) break
				}
				sendUserToast(`Resolved ${resolved} failed run${resolved === 1 ? '' : 's'}`)
			} catch (e) {
				sendUserToast(`Could not resolve the failed runs: ${e}`, true)
			}
			runs.refetch()
		}
	}
}

export type RecentRunsState = ReturnType<typeof useRecentRuns>

const PANEL_OPEN_SETTING = 'wm-home-runs-open'

export function usePanelOpen() {
	let open = $state(getLocalSetting(PANEL_OPEN_SETTING) !== 'false')
	return {
		get val() {
			return open
		},
		set val(v: boolean) {
			open = v
			try {
				storeLocalSetting(PANEL_OPEN_SETTING, v ? undefined : 'false')
			} catch {}
		}
	}
}

export function itemHref(item: Pick<RecentRunItem, 'kind' | 'path'>) {
	const base = item.kind === 'flow' ? 'flows' : item.kind === 'agent' ? 'agents' : 'scripts'
	return `/${base}/get/${item.path}`
}

export function runHref(id: string, workspace: string | undefined) {
	return `/run/${id}?workspace=${workspace ?? ''}`
}

export function allRunsHref(folder: string | undefined) {
	return folder ? `/runs?folder=${encodeURIComponent(folder)}` : '/runs'
}

export function windowLabel(hours: number) {
	return hours > 24 && hours % 24 === 0 ? `last ${hours / 24}d` : `last ${hours}h`
}

/** `now`, `3m`, `2h`, `4d`; `running` while the newest run has not finished. */
export function shortAge(run: RecentRun, now: number) {
	if (run.status === 'running') return 'running'
	const s = Math.max(0, (now - new Date(run.ran_at).getTime()) / 1000)
	if (s < 60) return 'now'
	if (s < 3600) return `${Math.floor(s / 60)}m`
	if (s < 86400) return `${Math.floor(s / 3600)}h`
	return `${Math.floor(s / 86400)}d`
}

/** A script and a flow can share a path: an item is both. */
export function itemKey(item: Pick<RecentRunItem, 'kind' | 'path'>) {
	return `${item.kind}:${item.path}`
}

export function itemTitle(item: Pick<RecentRunItem, 'summary' | 'path'>) {
	return item.summary || item.path.split('/').pop() || item.path
}

/** The folder (or user) to show before a title another listed item also carries. */
export function collisionPrefixes(items: RecentRunItem[]): Map<string, string> {
	const byTitle = new Map<string, number>()
	for (const item of items) byTitle.set(itemTitle(item), (byTitle.get(itemTitle(item)) ?? 0) + 1)
	const prefixes = new Map<string, string>()
	for (const item of items) {
		if ((byTitle.get(itemTitle(item)) ?? 0) > 1)
			prefixes.set(itemKey(item), item.path.split('/')[1])
	}
	return prefixes
}

export function totalLabel(runs: RecentRuns) {
	return `${runs.total_runs}${runs.capped ? '+' : ''} runs`
}

export type AskAIContext =
	| { kind: 'failing'; items: RecentRunItem[] }
	| { kind: 'run'; path: string; jobId: string }

/** Whether Home's composer is there to take a run: it registers only once mounted. */
export function canAskAI(): boolean {
	return homeComposerContext.available
}

/** Each run into Home's composer as context, `draft` before them: a failing item hands over its
 * latest run, which is the one that failed. */
export async function askAI(context: AskAIContext, draft?: string) {
	const runs =
		context.kind === 'run'
			? [{ path: context.path, jobId: context.jobId }]
			: context.items.flatMap((i) => (i.runs[0] ? [{ path: i.path, jobId: i.runs[0].id }] : []))
	// Mentions land at the caret, so they go in first; `prefill` then puts the question ahead of them.
	for (const run of runs) await homeComposerContext.attach(runContextElement(run.jobId, run.path))
	if (draft) homeComposerContext.prefill(`${draft} `)
}
