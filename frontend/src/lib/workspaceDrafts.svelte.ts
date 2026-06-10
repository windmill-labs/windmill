/**
 * Workspace Drafts — the single source of truth for "which Server Drafts exist
 * in a workspace". Lists the deployable Draft Items once; the Draft Count is
 * simply that list's length — never a separate query. This is what makes the
 * count reliable: count ≡ list, by construction.
 *
 * Behind this seam the list is currently assembled from the three version-aware
 * list endpoints (scripts/flows/apps with `include_draft_only`). A single
 * `GET /w/{ws}/drafts/items` endpoint can replace `getDraftItems` later without
 * touching any consumer.
 *
 * Reactivity: `useWorkspaceDrafts(() => ws)` is a component-scoped `runed`
 * resource — it fetches on mount and when `ws` changes, and is disposed on
 * unmount, so a re-opened view always shows a fresh count (no persistent cache
 * to go stale). `invalidateWorkspaceDrafts(ws)` bumps a per-workspace version so
 * every *mounted* consumer re-fetches after a Server-Draft mutation.
 */
import { resource } from 'runed'
import { ScriptService, FlowService, AppService } from '$lib/gen'

export type DraftKind = 'script' | 'flow' | 'app'

export interface DraftItem {
	kind: DraftKind
	path: string
	summary?: string
	/** Never deployed — exists only as a draft. */
	draft_only: boolean
	/** App is a raw app (deploys via the raw-app endpoints). Always false for non-apps. */
	raw_app: boolean
}

/** The one place the "is this a deployable Draft Item?" rule lives on the
 * frontend: a pending draft on a deployed item (`has_draft`) OR a never-deployed
 * `draft_only` item. Mirrors the backend `count_drafts` predicate. */
/** The list-endpoint fields this module reads. Kept as a narrow local interface
 * (rather than `any`) so the count predicate isn't typed against `any`. NOTE:
 * `openapi.yaml`'s `ListableApp` still omits `has_draft`/`draft_only` (the backend
 * struct returns them) — the proper fix is to add them to the spec and regenerate
 * the client; until then this interface documents the contract relied on. */
interface DraftListEntry {
	path: string
	summary?: string
	has_draft?: boolean
	draft_only?: boolean
	raw_app?: boolean
}

// The list endpoints are paginated; without paging, drafts past the first page
// would be silently missing from the count/list (and "Deploy all"). Page through
// with a generous page size until a short page signals the end.
const DRAFT_LIST_PER_PAGE = 100

async function listAllPages(
	fetchPage: (page: number, perPage: number) => Promise<DraftListEntry[]>
): Promise<DraftListEntry[]> {
	const all: DraftListEntry[] = []
	for (let page = 1; ; page++) {
		const batch = await fetchPage(page, DRAFT_LIST_PER_PAGE)
		all.push(...batch)
		if (batch.length < DRAFT_LIST_PER_PAGE) break
	}
	return all
}

export async function getDraftItems(workspace: string): Promise<DraftItem[]> {
	const [scripts, flows, apps] = await Promise.all([
		listAllPages((page, perPage) =>
			ScriptService.listScripts({ workspace, includeDraftOnly: true, page, perPage })
		),
		listAllPages((page, perPage) =>
			FlowService.listFlows({ workspace, includeDraftOnly: true, page, perPage })
		),
		listAllPages((page, perPage) =>
			AppService.listApps({ workspace, includeDraftOnly: true, page, perPage })
		)
	])
	const items: DraftItem[] = []
	const push = (kind: DraftKind, list: DraftListEntry[]) => {
		for (const it of list) {
			if (it.has_draft || it.draft_only) {
				items.push({
					kind,
					path: it.path,
					summary: it.summary,
					draft_only: !!it.draft_only,
					raw_app: !!it.raw_app
				})
			}
		}
	}
	push('script', scripts)
	push('flow', flows)
	push('app', apps)
	items.sort((a, b) => a.path.localeCompare(b.path))
	return items
}

// Per-workspace invalidation version. Bumping it changes the resource key for
// that workspace, so mounted consumers re-fetch. Plain $state record.
const versions: Record<string, number> = $state({})

export function invalidateWorkspaceDrafts(workspace: string | undefined): void {
	if (!workspace) return
	versions[workspace] = (versions[workspace] ?? 0) + 1
}

export interface WorkspaceDraftsHandle {
	readonly items: DraftItem[]
	readonly count: number
	readonly loading: boolean
	/** Imperative re-fetch (e.g. right after a mutation in the same component). */
	refresh: () => void
}

/**
 * Reactive Workspace Drafts for the given workspace. Call at component init.
 * Re-fetches on mount, when `workspace` changes, and when
 * `invalidateWorkspaceDrafts(workspace)` is called while mounted.
 */
export function useWorkspaceDrafts(workspace: () => string | undefined): WorkspaceDraftsHandle {
	const res = resource(
		() => {
			const ws = workspace()
			return { ws, v: ws ? (versions[ws] ?? 0) : 0 }
		},
		async ({ ws }) => (ws ? getDraftItems(ws) : [])
	)
	return {
		get items() {
			return res.current ?? []
		},
		get count() {
			return (res.current ?? []).length
		},
		get loading() {
			return res.loading
		},
		refresh() {
			void res.refetch()
		}
	}
}
