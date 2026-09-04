/**
 * One-off sweep that drops drafts carrying no changes.
 *
 * Before the editors gated their autosave on real user input (`onUserInput`),
 * merely opening an item whose schema had moved on saved a draft — workspaces
 * accumulated dozens that nobody wrote. The gate stops new ones; the ones
 * already stored need this pass to clear. Runs once per (workspace, user) per
 * browser, after the localStorage→DB migration so anything it just uploaded is
 * swept too.
 *
 * A draft is dropped only when the diff the user would be shown is empty: both
 * sides come from `getDraftDiffValues`, the same canonicalization the diff
 * drawer renders, compared with the same `draftValuesEqual` the editors use.
 * Anything that can't be established is left alone — a `draft_only` item (no
 * deployed counterpart, so discarding would destroy the item itself), a kind
 * with no diff support, a failed fetch.
 *
 * Deleting is the dangerous half, and the equality behind it is always stale:
 * it was read one round trip ago, and every candidate is read before any is
 * deleted. Two guards keep that from eating work. Anything this tab is
 * currently writing is skipped outright — a discard POSTs immediately, which
 * cancels the autosave the user's keystrokes have queued. And the delete
 * itself is a compare-and-delete: the listing's timestamp is seeded as the
 * `last_sync` baseline, so the backend refuses it if the row moved since,
 * whoever moved it. Without the seed a freshly loaded tab has no baseline and
 * the delete is unconditional.
 */

import { DraftService } from './gen'
import type { UserDraftItemKind } from './gen'
import { sendUserToast } from './toast'
import { UserDraft, draftValuesEqual } from './userDraft.svelte'
import { UserDraftDbSyncer } from './userDraftDbSyncer.svelte'
import { discardDraft, getDraftDiffValues } from './utils_draft_deploy'
import { invalidateWorkspaceDrafts } from './workspaceDrafts.svelte'

const SENTINEL_PREFIX = 'userdraft/pruned/v1/'

/** Overlay GETs are one round trip each and a cluttered workspace has dozens;
 * a small window keeps the sweep off the critical path of a fresh login. */
const CONCURRENCY = 4

/** Guards against the layout effect firing again before the sentinel lands. */
const inFlight = new Set<string>()

type Candidate = {
	kind: UserDraftItemKind
	path: string
	legacy: boolean
	/** The row's `created_at` as listed — the baseline the delete is conditioned on. */
	createdAt: string
}

/** Is this tab holding or writing this draft right now? */
function busyLocally(workspace: string, kind: UserDraftItemKind, path: string): boolean {
	if (UserDraft.has(kind, path, { workspace })) return true
	return UserDraftDbSyncer.getState({ workspace, itemKind: kind, path }).state !== 'none'
}

async function carriesNoChanges(workspace: string, { kind, path }: Candidate): Promise<boolean> {
	try {
		const { deployed, draft, hasDraft, noDeployed } = await getDraftDiffValues(
			kind,
			path,
			workspace
		)
		// `hasDraft` false means the overlay had no draft row and the item's own
		// value stood in for the draft side — there is nothing to discard, and the
		// two sides would compare equal by construction.
		if (!hasDraft || noDeployed) return false
		return draftValuesEqual(draft, deployed)
	} catch {
		return false
	}
}

async function mapWithLimit<T, R>(
	items: T[],
	limit: number,
	fn: (item: T) => Promise<R>
): Promise<R[]> {
	const out = new Array<R>(items.length)
	let next = 0
	await Promise.all(
		Array.from({ length: Math.min(limit, items.length) }, async () => {
			while (next < items.length) {
				const i = next++
				out[i] = await fn(items[i])
			}
		})
	)
	return out
}

export async function pruneMeaninglessDrafts(workspace: string, userKey: string): Promise<void> {
	if (typeof localStorage === 'undefined') return
	const sentinel = `${SENTINEL_PREFIX}${workspace}/${userKey}`
	if (inFlight.has(sentinel)) return
	try {
		if (localStorage.getItem(sentinel)) return
	} catch {
		// Storage unavailable (private mode): the sweep can't record that it ran,
		// and re-running it on every mount would cost an overlay GET per draft.
		return
	}
	inFlight.add(sentinel)
	try {
		const rows = await DraftService.listDrafts({ workspace })
		const candidates: Candidate[] = rows
			// `draft_only` rows ARE the item; `mine` / `can_write` are the same
			// gate the discard endpoint enforces, so anything else would 403.
			.filter((r) => !r.draft_only && r.mine && r.can_write)
			.filter((r) => !busyLocally(workspace, r.kind, r.path))
			.map((r) => ({
				kind: r.kind,
				path: r.path,
				legacy: r.legacy_draft,
				createdAt: r.created_at
			}))

		const empty: Candidate[] = []
		await mapWithLimit(candidates, CONCURRENCY, async (c) => {
			if (await carriesNoChanges(workspace, c)) empty.push(c)
		})

		let discarded = 0
		let failed = 0
		for (const c of empty) {
			// Re-check: the reads above took a while, and the user may have opened
			// this item in the meantime.
			if (busyLocally(workspace, c.kind, c.path)) continue
			const q = { workspace, itemKind: c.kind, path: c.path }
			UserDraftDbSyncer.recordRemoteSync(q, c.createdAt)
			const res = await discardDraft(c.kind, c.path, workspace, false, c.legacy, false)
			// Neither outcome throws: the syncer swallows an HTTP failure into its
			// per-key state, and a delete refused for a moved row comes back as a
			// conflict. So `success` alone says nothing about whether the row went.
			if (!res.success || UserDraftDbSyncer.getState(q).state === 'failed') failed++
			else if (!UserDraftDbSyncer.getConflict(q).conflict) discarded++
		}
		if (discarded > 0) {
			invalidateWorkspaceDrafts(workspace)
			sendUserToast(`Cleared ${discarded} draft${discarded > 1 ? 's' : ''} that carried no changes`)
		}
		// Only once every deletion this pass attempted actually landed. A draft
		// left behind by a failed delete would otherwise never be revisited.
		if (failed === 0) {
			try {
				localStorage.setItem(sentinel, new Date().toISOString())
			} catch {
				// Nothing to do — the pass is idempotent, it just runs again.
			}
		}
	} catch {
		// Fire-and-forget from the layout: a workspace whose draft list can't be
		// read is left exactly as it was.
	} finally {
		inFlight.delete(sentinel)
	}
}
