import { JobService, type ListRecentRunsResponse } from '$lib/gen'
import { getLocalSetting, storeLocalSetting } from '$lib/utils'
import { IsDocumentVisible, resource, useInterval } from 'runed'
import { homeComposerContext } from '$lib/components/sessions/homeComposerContext.svelte'
import { runContextElement } from '$lib/components/copilot/chat/context'

export type RecentRuns = ListRecentRunsResponse
export type RecentRunItem = RecentRuns['items'][number]
export type RecentRun = RecentRunItem['runs'][number]
export type RecentRunStatus = RecentRun['status']

/** The Runs page's own refresh rate (`useJobsLoader`), so both read the database as often. */
const REFRESH_MS = 5000

/**
 * What ran recently in the workspace, read from `listRecentRuns`. The side panel and the header
 * status read the same instance: the counts they show must never disagree. Nothing is read while
 * `enabled` is false: each read scans the window's runs.
 */
export function useRecentRuns(
	args: () => { workspace: string | undefined; folder?: string; enabled: boolean }
) {
	const runs = resource(args, async ({ workspace, folder, enabled }, _, { data, refetching }) => {
		if (!workspace || !enabled) return undefined
		try {
			return await JobService.listRecentRuns({ workspace, folder })
		} catch {
			// A background refresh that fails (a backend restart) keeps what is on screen rather
			// than surfacing an error every few seconds; a failed first load shows nothing.
			return refetching ? data : undefined
		}
	})
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
		get now() {
			return now
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
	// Mentions land at the caret, so the question goes in after them, ahead of the draft.
	for (const run of runs) await homeComposerContext.attach(runContextElement(run.jobId, run.path))
	if (draft) homeComposerContext.prefill(`${draft} `)
}
