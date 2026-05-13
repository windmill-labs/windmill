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

function matchLegacyKey(
	key: string
): { prefix: string; newKind: LegacyKind; path: string } | undefined {
	for (const { prefix, newKind } of LEGACY_PREFIXES) {
		if (key === prefix) return { prefix, newKind, path: '' }
		if (key.startsWith(prefix + '-')) {
			return { prefix, newKind, path: key.slice(prefix.length + 1) }
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
				if (decoded == null || typeof decoded !== 'object') continue
				const value = transformLegacyValue(match.newKind, decoded)
				const target = newKey(workspace, match.newKind, match.path)
				if (value !== undefined && localStorage.getItem(target) == null) {
					localStorage.setItem(target, JSON.stringify({ value }))
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
