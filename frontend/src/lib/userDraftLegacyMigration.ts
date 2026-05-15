/**
 * One-off migration from the pre-UserDraft localStorage autosave entries to
 * the workspace-scoped `userdraft/w/{ws}/{kind}/{path}` format.
 *
 * Legacy keys (global, not workspace-scoped — assumed to belong to the user's
 * current workspace at migration time):
 *
 *   `flow`            / `flow-{path}`    base64 of `encodeState({ flow, path, selectedId, draft_triggers, ... })`
 *   `app`             / `app-{path}`     base64 of `encodeState(App)`
 *   `rawapp`          / `rawapp-{path}`  base64 of `encodeState({ files, runnables, data })`
 *
 * Target keys: `userdraft/w/{workspace}/{flow|app|raw_app}/{path}` storing
 * `JSON.stringify({ value: <transformed legacy value> })`.
 *
 * Idempotent: writes a sentinel under `MIGRATION_FLAG` after the first run so
 * subsequent invocations are no-ops. Existing new-format entries are never
 * overwritten — when both an old and a new entry exist for the same item, the
 * old one is simply dropped on the assumption that the new entry is the more
 * recent edit.
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
 * Windmill draft (any base64-of-JSON could pass). Promoting a stray payload
 * would silently surface as a phantom "Restored from local storage" toast
 * on the next edit, so we reject anything that doesn't carry the fields the
 * legacy writers actually produced.
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

function transformLegacyValue(kind: LegacyKind, decoded: unknown): unknown {
	const obj = decoded as Record<string, unknown>
	switch (kind) {
		case 'flow':
			// The legacy bundle wrapped the Flow alongside view-state fields
			// (selectedId, draft_triggers, ...). The new entry stores only the
			// Flow — the view-state lives elsewhere or is re-derived.
			return obj.flow
		case 'app':
			// Legacy stored the App directly.
			return obj
		case 'raw_app':
			// Legacy bundle missed the `summary` field that the new editor adds.
			return {
				files: obj.files ?? {},
				runnables: obj.runnables ?? {},
				data: obj.data ?? {},
				summary: typeof obj.summary === 'string' ? obj.summary : ''
			}
	}
}

function newKey(workspace: string, kind: LegacyKind, path: string): string {
	return `userdraft/w/${workspace}/${kind}/${path}`
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
 * Run the legacy → new-format migration. Idempotent: returns immediately if a
 * previous run completed (signalled by `MIGRATION_FLAG`).
 *
 * The migration is workspace-scoped because the legacy keys had no notion of
 * workspace — we treat the caller's current workspace as the owner of any
 * surviving legacy entries.
 */
export function migrateLegacyUserDrafts(workspace: string): void {
	if (typeof localStorage === 'undefined') return
	if (!workspace) return
	if (localStorage.getItem(MIGRATION_FLAG) !== null) return

	try {
		for (const key of listLocalStorageKeys()) {
			const match = matchLegacyKey(key)
			if (!match) continue
			const raw = localStorage.getItem(key)
			if (raw == null) continue

			try {
				const decoded = decodeLegacyState(raw)
				if (!isPlausibleLegacyValue(match.newKind, decoded)) continue
				const value = transformLegacyValue(match.newKind, decoded)
				const target = newKey(workspace, match.newKind, match.path)
				if (value !== undefined && localStorage.getItem(target) == null) {
					// `lastWrittenAt` makes the migrated entry visible to
					// `gcUserDrafts`. We stamp it as "now" so a freshly-migrated
					// autosave gets the full retention window — sweeping it
					// immediately on the first GC pass would lose work the
					// legacy migration just rescued.
					localStorage.setItem(target, JSON.stringify({ value, lastWrittenAt: Date.now() }))
				}
				localStorage.removeItem(key)
			} catch (e) {
				console.error('UserDraft legacy migration: failed to migrate', key, e)
			}
		}
		localStorage.setItem(MIGRATION_FLAG, new Date().toISOString())
	} catch (e) {
		console.error('UserDraft legacy migration: aborted', e)
	}
}

/** Test-only: clear the sentinel so the migration can re-run. */
export function __resetUserDraftLegacyMigrationForTesting(): void {
	try {
		localStorage.removeItem(MIGRATION_FLAG)
	} catch {
		// ignore
	}
}
