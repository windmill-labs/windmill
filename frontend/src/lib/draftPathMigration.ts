/**
 * Run-once migration for drafts written by the pre-`draft_path`-removal code.
 *
 * Old editors stored a staged rename in the draft JSON's `draft_path` while
 * `path` held the deployed/storage path (flows/apps/raw-apps); scripts carried a
 * redundant `draft_path` alongside their already-correct `path`. The new code
 * reads the rename from the draft's own `path` everywhere (home/Review&Deploy
 * lists, deploy), so without migrating, an in-flight rename draft would lose its
 * friendly name from the lists AND deploy at the old path (silently dropping the
 * rename).
 *
 * This rewrites each of the current user's drafts in place: `path = draft_path`
 * (when set), then drops `draft_path`. Frontend-only, idempotent, guarded by a
 * per-workspace localStorage flag so it runs at most once per browser/workspace.
 * The original `created_at` is preserved so migrated drafts don't resurface to
 * the top of the list.
 */
import { DraftService, type UserDraftItemKind } from '$lib/gen'

// Only the full-page-editor kinds ever wrote a `draft_path` (scripts redundantly);
// drawer kinds (variable/resource/triggers) never did, so skip them — avoids
// needless GETs and never re-saves a secret variable's encrypted value.
const MIGRATED_KINDS: UserDraftItemKind[] = ['script', 'flow', 'app', 'raw_app']

const flagKey = (workspace: string) => `wm:draftPathMigration:v1:${workspace}`

// In-memory guard so concurrent callers in one session don't run it twice before
// the localStorage flag is written.
const inFlight = new Set<string>()

function alreadyDone(workspace: string): boolean {
	try {
		return localStorage.getItem(flagKey(workspace)) != null
	} catch {
		// No localStorage (SSR / privacy mode): treat as "done" so we never loop.
		return true
	}
}

/**
 * Migrate the current user's `draft_path` drafts in `workspace`. Resolves to
 * `true` when at least one draft was rewritten (caller can refresh the list).
 * Never throws. The run-once flag is set only on a fully clean pass: if the
 * list fetch OR any individual draft rewrite fails, the flag stays unset so a
 * later load retries (these are the user's own drafts, so a write failure is
 * transient rather than a permission denial).
 */
export async function migrateOwnDraftPaths(workspace: string): Promise<boolean> {
	if (!workspace || inFlight.has(workspace) || alreadyDone(workspace)) return false
	inFlight.add(workspace)
	let changed = false
	let hadFailure = false
	try {
		const rows = await DraftService.listDrafts({ workspace })
		const candidates = rows.filter((r) => r.mine !== false && MIGRATED_KINDS.includes(r.kind))
		for (const r of candidates) {
			try {
				const own = await DraftService.getOwnDraft({ workspace, kind: r.kind, path: r.path })
				const value = own?.value
				if (!value || typeof value !== 'object' || Array.isArray(value)) continue
				const v = value as Record<string, unknown>
				if (!('draft_path' in v)) continue
				const dp = v.draft_path
				const { draft_path: _drop, ...rest } = v
				const next = typeof dp === 'string' && dp !== '' ? { ...rest, path: dp } : rest
				await DraftService.updateDraft({
					workspace,
					kind: r.kind,
					path: r.path,
					requestBody: { value: next, force: true, created_at: own?.created_at }
				})
				changed = true
			} catch (e) {
				// Skip this draft but don't abort the pass; leave the flag unset so it
				// retries on a later load.
				hadFailure = true
				console.warn('draft_path migration: skipped', r.kind, r.path, e)
			}
		}
		if (!hadFailure) {
			try {
				localStorage.setItem(flagKey(workspace), new Date().toISOString())
			} catch {
				// best-effort
			}
		}
	} catch (e) {
		// Listing failed — leave the flag unset so a later load retries.
		console.warn('draft_path migration failed', e)
	} finally {
		inFlight.delete(workspace)
	}
	return changed
}
