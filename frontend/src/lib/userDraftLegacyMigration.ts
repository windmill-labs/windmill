/**
 * One-shot purge of the pre-UserDraft browser-local autosave keys.
 *
 * The original autosave (pre-#9121) wrote workspace-BLIND keys:
 *
 *   `flow`   / `flow-{path}`    base64 of `encodeState({ flow, path, selectedId, draft_triggers, ... })`
 *   `app`    / `app-{path}`     base64 of `encodeState(App)`
 *   `rawapp` / `rawapp-{path}`  base64 of `encodeState({ files, runnables, data })`
 *
 * Neither the key nor the decoded value records a workspace (the value carries
 * only workspace-agnostic item paths like `u/me/x`), so these drafts cannot be
 * attributed to the workspace they were edited in. The current editors are
 * DB-backed and never read these keys, so they are dead data with one dangerous
 * property: promoting them to the DB would force a GUESS of the workspace, which
 * mis-files drafts into whatever workspace happened to be active when the
 * migration first ran (a single global sentinel gates it). We therefore drop
 * them instead of migrating them.
 *
 * Only keys that BOTH match the legacy path shape AND decode to a plausible
 * legacy draft are removed; unrelated look-alikes (`app-recent`, garbage,
 * non-Windmill payloads) are left untouched. The workspace-scoped interim keys
 * (`userdraft/w/{ws}/...`, written by the editor with the correct workspace)
 * are NOT touched here — `migrateUserDraftsToDb` still pushes those to the DB.
 *
 * Idempotent: writes a sentinel under `MIGRATION_FLAG` after the first run so
 * subsequent invocations are no-ops.
 *
 * This file is intentionally standalone — it does not import from
 * `userDraft.svelte.ts` so the new code stays uncluttered by the legacy
 * decoders.
 */

const MIGRATION_FLAG = 'userdraft/legacy_migrated_v1'

type LegacyKind = 'flow' | 'app' | 'raw_app'

const LEGACY_PREFIXES: ReadonlyArray<{ prefix: string; newKind: LegacyKind }> = [
	// `rawapp` is listed before `app` even though our matcher uses exact /
	// dash-separated comparison (so there's no ambiguity); it documents the
	// intent that raw apps are a distinct kind, not a sub-case of apps.
	{ prefix: 'rawapp', newKind: 'raw_app' },
	{ prefix: 'flow', newKind: 'flow' },
	{ prefix: 'app', newKind: 'app' }
]

/**
 * A Windmill item path: `u/<owner>/<name…>` or `f/<folder>/<name…>`. The
 * `<name…>` segment may itself contain slashes, so we don't constrain it
 * past requiring at least one character. Used to reject incidentally-named
 * localStorage keys (e.g. `app-recent` from a future feature, or a
 * neighbouring app's data) before treating them as Windmill drafts.
 */
const LEGACY_PATH_SHAPE = /^[uf]\/[^/]+\/.+$/

function matchLegacyKey(
	key: string
): { prefix: string; newKind: LegacyKind; path: string } | undefined {
	for (const { prefix, newKind } of LEGACY_PREFIXES) {
		if (key === prefix) return { prefix, newKind, path: '' }
		if (key.startsWith(prefix + '-')) {
			const path = key.slice(prefix.length + 1)
			if (!LEGACY_PATH_SHAPE.test(path)) return undefined
			return { prefix, newKind, path }
		}
	}
	return undefined
}

function decodeLegacyState(raw: string): unknown {
	try {
		return JSON.parse(decodeURIComponent(atob(raw)))
	} catch {
		return undefined
	}
}

/**
 * Per-kind shape gate. The legacy keys (`app-foo`, `flow-foo`, ...) are
 * unusual enough that nothing else in the codebase has used them, but
 * matching `LEGACY_PATH_SHAPE` doesn't prove the payload is actually a
 * Windmill draft (any base64-of-JSON could pass). We only delete keys we can
 * positively recognise as legacy drafts, so a stray look-alike that happens to
 * use this key shape is left untouched rather than silently dropped.
 */
function isPlausibleLegacyValue(kind: LegacyKind, decoded: unknown): boolean {
	if (decoded == null || typeof decoded !== 'object') return false
	const obj = decoded as Record<string, unknown>
	switch (kind) {
		case 'flow':
			// Legacy FlowBuilder wrote { flow, path, selectedId, draft_triggers, ... }.
			return obj.flow != null && typeof obj.flow === 'object'
		case 'app':
			// Legacy AppEditor wrote `encodeState($appStore)`, i.e. the inner App
			// value (see `frontend/src/lib/components/apps/types.ts`) — NOT the
			// wrapping AppWithLastVersion. It carries `grid`, `fullscreen`,
			// `theme`, `unusedInlineScripts`, `hiddenInlineScripts` among other
			// fields — any one of those is a strong signal it's actually a
			// Windmill app payload.
			return (
				'grid' in obj ||
				'fullscreen' in obj ||
				'theme' in obj ||
				'unusedInlineScripts' in obj ||
				'hiddenInlineScripts' in obj
			)
		case 'raw_app':
			// Legacy RawAppEditor wrote { files, runnables, data }.
			return 'files' in obj || 'runnables' in obj || 'data' in obj
	}
}

function listLocalStorageKeys(): string[] {
	const out: string[] = []
	for (let i = 0; i < localStorage.length; i++) {
		const k = localStorage.key(i)
		if (k != null) out.push(k)
	}
	return out
}

/**
 * Remove the workspace-blind legacy autosave keys (see file header). Idempotent:
 * returns immediately if a previous run completed (signalled by `MIGRATION_FLAG`).
 */
export function purgeLegacyUserDrafts(): void {
	if (typeof localStorage === 'undefined') return
	if (localStorage.getItem(MIGRATION_FLAG) !== null) return

	try {
		for (const key of listLocalStorageKeys()) {
			const match = matchLegacyKey(key)
			if (!match) continue
			const raw = localStorage.getItem(key)
			if (raw == null) continue
			// Only drop keys we can positively recognise as legacy Windmill
			// drafts; leave unrelated or unparseable look-alikes in place.
			if (!isPlausibleLegacyValue(match.newKind, decodeLegacyState(raw))) continue
			localStorage.removeItem(key)
		}
		localStorage.setItem(MIGRATION_FLAG, new Date().toISOString())
	} catch (e) {
		console.error('UserDraft legacy purge: aborted', e)
	}
}

/** Test-only: clear the sentinel so the purge can re-run. */
export function __resetUserDraftLegacyMigrationForTesting(): void {
	try {
		localStorage.removeItem(MIGRATION_FLAG)
	} catch {
		// ignore
	}
}
