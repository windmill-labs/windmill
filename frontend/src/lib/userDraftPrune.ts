/**
 * One-off sweep that drops drafts carrying no changes.
 *
 * `onUserInput` stops new ones from being written; the ones already stored need
 * this pass to clear. Runs once per (workspace, user) per browser, after the
 * localStorage→DB migration so anything it just uploaded is swept too.
 *
 * Scoped to the kinds whose editors that gate covers. A script, flow or app
 * draft only ever came from an explicit edit, so there is no phantom to clear
 * there — and `getDraftDiffValues` would fetch each one's full deployed payload
 * at login to prove it.
 *
 * A draft is dropped only when the diff the user would be shown is empty: both
 * sides come from `getDraftDiffValues`, the same canonicalization the diff
 * drawer renders, compared with the same `draftValuesEqual` the editors use.
 * Anything that can't be established is left alone — a `draft_only` item (no
 * deployed counterpart, so discarding would destroy the item itself), a kind
 * with no diff support, a failed fetch, and the legacy workspace-level rows,
 * which belong to nobody and are admin-gated to migrate.
 *
 * Deleting is the dangerous half, and the equality behind it is always stale:
 * it was read one round trip ago, and every candidate is read before any is
 * deleted. So the delete is a compare-and-delete — `last_sync` is the
 * timestamp the row was judged on, and the backend drops it only if nothing has
 * written it since, whoever wrote it.
 *
 * It is sent straight to `DraftService`, NOT through `UserDraftDbSyncer`. That
 * syncer exists to autosave an editor's own live value, and everything it does
 * for that — parking the payload for the `pagehide` flush, debouncing, holding
 * a per-tab `last_sync` baseline and conflict state — is a way for a one-shot
 * delete to reach back into whatever the user is doing in the same tab. This
 * sweep wants exactly one conditional request and no state afterwards.
 */

import { DraftService } from './gen'
import type { UserDraftItemKind } from './gen'
import { sendUserToast } from './toast'
import { setLocalDraftHint } from './localDraftHints.svelte'
import { UserDraft, draftValuesEqual } from './userDraft.svelte'
import { UserDraftDbSyncer } from './userDraftDbSyncer.svelte'
import { canDiffDraftKind, getDraftDiffValues } from './utils_draft_deploy'
import { invalidateWorkspaceDrafts } from './workspaceDrafts.svelte'

const SENTINEL_PREFIX = 'userdraft/pruned/v1/'

/** A pass that leaves anything unresolved runs again next mount, which costs a
 * listing plus an overlay GET per row. Bounded so no permanently-unresolvable
 * row can make that repeat forever — whatever the reason it can't be judged. */
const MAX_PASSES = 3

/** The editors whose forms are built from a schema, and so the only kinds that
 * could have banked a draft nobody wrote — minus the ones no diff can be
 * computed for, which would throw and keep the pass unsealed forever. */
function isSweepableKind(kind: UserDraftItemKind): boolean {
	return (kind === 'resource' || kind.startsWith('trigger_')) && canDiffDraftKind(kind)
}

/** Overlay GETs are one round trip each and a cluttered workspace has dozens;
 * a small window keeps the sweep off the critical path of a fresh login. */
const CONCURRENCY = 4

/** Guards against the layout effect firing again before the sentinel lands. */
const inFlight = new Set<string>()

type Candidate = {
	kind: UserDraftItemKind
	path: string
	/** The row's `created_at` as listed — the baseline the delete is conditioned on. */
	createdAt: string
}

const attemptsKey = (sentinel: string) => `${sentinel}:attempts`

function readAttempts(sentinel: string): number {
	try {
		const n = Number(localStorage.getItem(attemptsKey(sentinel)))
		return Number.isFinite(n) && n > 0 ? n : 0
	} catch {
		return 0
	}
}

/** Is this tab holding or writing this draft right now? */
function busyLocally(workspace: string, kind: UserDraftItemKind, path: string): boolean {
	if (UserDraft.has(kind, path, { workspace })) return true
	return UserDraftDbSyncer.getState({ workspace, itemKind: kind, path }).state !== 'none'
}

/** A 4xx is the server's final answer for this row — the item is gone, or the
 * kind's overlay endpoint isn't served by this build (a feature-gated trigger
 * on CE). Retrying it on every page load would never succeed. Anything else
 * (network, 5xx) is worth another pass. 429 asks for exactly that. */
function isPermanentlyUnjudgeable(e: unknown): boolean {
	const status = (e as { status?: unknown })?.status
	return typeof status === 'number' && status >= 400 && status < 500 && status !== 429
}

/** `undefined` when the diff could not be fetched and might be next time —
 * distinct from `false`, so the caller can leave the pass open rather than
 * strand a row it never judged. */
async function carriesNoChanges(
	workspace: string,
	{ kind, path }: Candidate
): Promise<boolean | undefined> {
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
	} catch (e) {
		return isPermanentlyUnjudgeable(e) ? false : undefined
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
		// Anything left unresolved keeps the pass open: a row skipped as busy was
		// never judged, and one whose delete failed is still there. Sealing on
		// either would strand it.
		let unresolved = 0
		const candidates: Candidate[] = rows
			// `draft_only` rows ARE the item; `mine` / `can_write` are the same
			// gate the discard endpoint enforces, so anything else would 403.
			.filter((r) => !r.draft_only && r.mine && r.can_write)
			.filter((r) => isSweepableKind(r.kind) && !r.legacy_draft)
			.filter((r) => {
				if (!busyLocally(workspace, r.kind, r.path)) return true
				unresolved++
				return false
			})
			.map((r) => ({
				kind: r.kind,
				path: r.path,
				createdAt: r.created_at
			}))

		const empty: Candidate[] = []
		await mapWithLimit(candidates, CONCURRENCY, async (c) => {
			const verdict = await carriesNoChanges(workspace, c)
			if (verdict === undefined) unresolved++
			else if (verdict) empty.push(c)
		})

		let discarded = 0
		for (const c of empty) {
			// Re-check: the reads above took a while, and the user may have opened
			// this item in the meantime.
			if (busyLocally(workspace, c.kind, c.path)) {
				unresolved++
				continue
			}
			try {
				const resp = await DraftService.updateDraft({
					workspace,
					kind: c.kind,
					path: c.path,
					requestBody: { value: null, last_sync: c.createdAt, force: false }
				})
				// `conflict` means the row moved past the timestamp we judged it on,
				// so it is no longer the empty draft we decided to drop.
				if (resp.status === 'saved') {
					setLocalDraftHint(workspace, c.kind, c.path, false)
					discarded++
				}
			} catch {
				unresolved++
			}
		}
		if (discarded > 0) {
			invalidateWorkspaceDrafts(workspace)
			sendUserToast(`Cleared ${discarded} draft${discarded > 1 ? 's' : ''} that carried no changes`)
		}
		// Seal once nothing is left hanging, or once we have tried enough times
		// that whatever is hanging is not going to resolve.
		const attempts = readAttempts(sentinel) + 1
		if (unresolved === 0 || attempts >= MAX_PASSES) {
			try {
				localStorage.setItem(sentinel, new Date().toISOString())
				localStorage.removeItem(attemptsKey(sentinel))
			} catch {
				// Nothing to do — the pass is idempotent, it just runs again.
			}
		} else {
			try {
				localStorage.setItem(attemptsKey(sentinel), String(attempts))
			} catch {}
		}
	} catch {
		// Fire-and-forget from the layout: a workspace whose draft list can't be
		// read is left exactly as it was.
	} finally {
		inFlight.delete(sentinel)
	}
}
