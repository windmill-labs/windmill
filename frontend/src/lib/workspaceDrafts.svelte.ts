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
	/** User-typed friendly path (from the draft JSON's `draft_path`) when it
	 * differs from the storage `path` — e.g. a never-deployed item parked at
	 * `u/{user}/draft_{uuid}`. Display this instead of `path` when present. */
	draft_path?: string
	/** Never deployed — exists only as a draft. */
	draft_only: boolean
	/** Legacy workspace-level draft (email NULL) predating the per-user drafts
	 * migration. Not tied to any user, so anyone with access to the path sees it. */
	legacy_draft: boolean
	/** App is a raw app (deploys via the raw-app endpoints). Always false for non-apps. */
	raw_app: boolean
	/** Current user may deploy/discard this draft — matches the server-side check.
	 * Defaults to true when the field is absent (older backend) so a frontend
	 * running ahead of the API doesn't disable every action; the deploy/discard
	 * endpoints enforce permission regardless. */
	can_write: boolean
	/** Draft authors at this (path, kind); populated only for the shared
	 * full-page-editor kinds (script/flow/app/raw_app). Feeds the badge circles. */
	draft_users?: { username?: string | null }[]
	/** The row is the current user's own draft (or the legacy no-owner row), so
	 * they can deploy/discard it. Always true in the default listing; only the
	 * `allUsers` listing surfaces other users' rows as `false` (view-only).
	 * Defaults to true when the field is absent (older backend). */
	mine: boolean
	/** Server timestamp of the draft row, bumped on every draft update — a
	 * reliable per-row change marker for caches keyed on draft content. */
	created_at: string
}

export async function getDraftItems(
	workspace: string,
	allUsers: boolean = false
): Promise<DraftItem[]> {
	const rows = await DraftService.listDrafts({ workspace, allUsers: allUsers || undefined })
	return rows.map((r) => ({
		kind: r.kind,
		path: r.path,
		summary: r.summary,
		draft_path: r.draft_path,
		draft_only: r.draft_only,
		legacy_draft: r.legacy_draft,
		raw_app: r.kind === 'raw_app',
		can_write: r.can_write ?? true,
		draft_users: r.draft_users,
		mine: r.mine ?? true,
		created_at: r.created_at
	}))
}

// Per-workspace invalidation version. Bumping it changes the resource key for
// that workspace, so mounted consumers re-fetch. Plain $state record.
const versions: Record<string, number> = $state({})

export function invalidateWorkspaceDrafts(workspace: string | undefined): void {
	if (!workspace) return
	versions[workspace] = (versions[workspace] ?? 0) + 1
}

/** Current invalidation version for a workspace. Non-reactive read — callers
 * compare it against a value captured earlier to detect Server-Draft mutations
 * (any deploy/discard/draft write that called `invalidateWorkspaceDrafts`). */
export function getWorkspaceDraftsVersion(workspace: string): number {
	return versions[workspace] ?? 0
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
export function useWorkspaceDrafts(
	workspace: () => string | undefined,
	allUsers: () => boolean = () => false
): WorkspaceDraftsHandle {
	const res = resource(
		() => {
			const ws = workspace()
			return { ws, all: allUsers(), v: ws ? (versions[ws] ?? 0) : 0 }
		},
		async ({ ws, all }) => (ws ? getDraftItems(ws, all) : [])
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
