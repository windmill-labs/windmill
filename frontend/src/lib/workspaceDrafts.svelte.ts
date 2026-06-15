/**
 * Workspace Drafts — the single source of truth for "which Server Drafts exist
 * in a workspace". Lists the deployable Draft Items once; the Draft Count is
 * simply that list's length — never a separate query. This is what makes the
 * count reliable: count ≡ list, by construction.
 *
 * Backed by `GET /w/{ws}/drafts/list` — one query over the `draft` table on
 * the server, covering every kind (scripts, flows, apps, variables, resources,
 * schedules, triggers) with a per-kind `draft_only` flag.
 *
 * Reactivity: `useWorkspaceDrafts(() => ws)` is a component-scoped `runed`
 * resource — it fetches on mount and when `ws` changes, and is disposed on
 * unmount, so a re-opened view always shows a fresh count (no persistent cache
 * to go stale). `invalidateWorkspaceDrafts(ws)` bumps a per-workspace version so
 * every *mounted* consumer re-fetches after a Server-Draft mutation.
 */
import { resource } from 'runed'
import { DraftService, type UserDraftItemKind } from '$lib/gen'

export type DraftKind = UserDraftItemKind

export interface DraftItem {
	kind: DraftKind
	path: string
	summary?: string
	/** Never deployed — exists only as a draft. */
	draft_only: boolean
	/** App is a raw app (deploys via the raw-app endpoints). Always false for non-apps. */
	raw_app: boolean
}

export async function getDraftItems(workspace: string): Promise<DraftItem[]> {
	const rows = await DraftService.listDrafts({ workspace })
	return rows.map((r) => ({
		kind: r.kind,
		path: r.path,
		summary: r.summary,
		draft_only: r.draft_only,
		raw_app: r.kind === 'raw_app'
	}))
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
