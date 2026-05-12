import { get } from 'svelte/store'
import { onDestroy } from 'svelte'
import { workspaceStore } from './stores'
import { useLocalStorageValue } from './svelte5Utils.svelte'

export type UserDraftItemKind =
	| 'script'
	| 'flow'
	| 'app'
	| 'raw_app'
	| 'resource'
	| 'variable'
	| 'trigger_schedule'
	| 'trigger_webhook'
	| 'trigger_default_email'
	| 'trigger_email'
	| 'trigger_http'
	| 'trigger_websocket'
	| 'trigger_postgres'
	| 'trigger_kafka'
	| 'trigger_nats'
	| 'trigger_mqtt'
	| 'trigger_sqs'
	| 'trigger_gcp'
	| 'trigger_azure'
	| 'trigger_poll'
	| 'trigger_cli'
	| 'trigger_nextcloud'
	| 'trigger_google'
	| 'trigger_github'

export type UserDraftOptions = {
	workspace?: string
}

export type UserDraftUseOptions<V> = UserDraftOptions & {
	/**
	 * Initial value used when localStorage holds no draft for this
	 * (workspace, itemKind, path). It is *not* eagerly persisted — the first
	 * actual mutation is what writes to localStorage.
	 */
	defaultValue?: V
}

/**
 * Snapshot of the remote item's freshness at the moment the local draft was
 * written. Used by editor routes to detect that the remote has moved on
 * (someone else deployed, or saved a DB draft) so we can warn the user
 * before they push stale changes.
 *
 * - `remoteRev`: the deployed version's id/hash/timestamp at draft creation.
 * - `remoteDraftRev`: the DB-draft `created_at` at draft creation, only set
 *   for kinds that have a DB-draft (`script`, `flow`, `app`, `raw_app`).
 */
export type UserDraftMeta = {
	remoteRev?: string | number
	remoteDraftRev?: string | number
}

/**
 * The shape of what we actually persist. Wrapping the value lets us add
 * metadata (timestamps, originating user, schema version, ...) later
 * without breaking existing entries.
 */
type StoredDraft<V> = { value: V } & UserDraftMeta

type DraftState<V> = { val: StoredDraft<V> | undefined }

type DraftEntry = {
	count: number
	state: DraftState<unknown>
}

const entries = new Map<string, DraftEntry>()

function resolveWorkspace(opts?: UserDraftOptions): string {
	const ws = opts?.workspace ?? get(workspaceStore)
	if (!ws) {
		throw new Error(
			'UserDraft: no workspace available (pass opts.workspace or set $workspaceStore)'
		)
	}
	return ws
}

/**
 * Returns true when this (workspace, itemKind, path) should never touch
 * localStorage. An empty path means "new item, not yet on disk"; we keep the
 * draft in-memory so multiple components on the same /add page still share
 * state, but we don't persist it to avoid colliding new-item drafts.
 */
function isLocalOnly(path: string): boolean {
	return path === ''
}

function wrap<V>(value: V | undefined, meta?: UserDraftMeta): StoredDraft<V> | undefined {
	if (value === undefined) return undefined
	const out: StoredDraft<V> = { value }
	if (meta?.remoteRev !== undefined) out.remoteRev = meta.remoteRev
	if (meta?.remoteDraftRev !== undefined) out.remoteDraftRev = meta.remoteDraftRev
	return out
}

function unwrap<V>(stored: StoredDraft<V> | undefined): V | undefined {
	return stored?.value
}

function extractMeta(stored: StoredDraft<unknown> | undefined): UserDraftMeta {
	if (!stored) return {}
	const meta: UserDraftMeta = {}
	if (stored.remoteRev !== undefined) meta.remoteRev = stored.remoteRev
	if (stored.remoteDraftRev !== undefined) meta.remoteDraftRev = stored.remoteDraftRev
	return meta
}

function readPersisted<V>(key: string): StoredDraft<V> | undefined {
	try {
		const raw = localStorage.getItem(key)
		if (raw == null || raw === 'undefined') return undefined
		const parsed = JSON.parse(raw)
		// Defensive: drop entries written before the wrapping migration. Their
		// raw payload doesn't have a `.value` and would surface as undefined
		// anyway — we just don't want to confuse `has()` callers.
		if (parsed == null || typeof parsed !== 'object' || !('value' in parsed)) return undefined
		return parsed as StoredDraft<V>
	} catch (e) {
		console.error('UserDraft: localStorage read failed', e)
		return undefined
	}
}

function createInMemoryState<V>(defaultValue: StoredDraft<V> | undefined): DraftState<V> {
	let s = $state<StoredDraft<V> | undefined>(defaultValue)
	return {
		get val(): StoredDraft<V> | undefined {
			return s
		},
		set val(newVal: StoredDraft<V> | undefined) {
			s = newVal
		}
	}
}

function mapKey(workspace: string, itemKind: UserDraftItemKind, path: string): string {
	return `${workspace}/${itemKind}/${path}`
}

function localStorageKey(workspace: string, itemKind: UserDraftItemKind, path: string): string {
	return `userdraft/w/${workspace}/${itemKind}/${path}`
}

export type UserDraftHandle<V> = {
	get draft(): V | undefined
	set draft(value: V | undefined)
	/**
	 * Read the rev metadata stored alongside the current draft. Empty object
	 * if the entry has no draft or no rev was ever recorded.
	 */
	get meta(): UserDraftMeta
	/**
	 * Atomically set the draft value AND its rev metadata in a single write.
	 *
	 * Used by editor routes to record the backend rev at load time without
	 * triggering an extra persist (combined with the value write, the
	 * underlying useLocalStorageValue's saveInitialValue=false dedup skips
	 * it). Subsequent `handle.draft = X` writes only mutate `value` and
	 * preserve whatever rev metadata is in place.
	 */
	setDraftAndMeta(value: V | undefined, meta: UserDraftMeta): void
	/**
	 * Update the rev metadata in place without touching the draft value.
	 * Used after the "Keep current draft" modal action to ack the new remote
	 * rev so the staleness modal doesn't fire again until the remote moves
	 * further.
	 */
	setMeta(meta: UserDraftMeta): void
}

export const UserDraft = {
	save<V>(itemKind: UserDraftItemKind, path: string, value: V, opts?: UserDraftOptions): void {
		const ws = resolveWorkspace(opts)
		const mk = mapKey(ws, itemKind, path)
		const entry = entries.get(mk)
		if (entry) {
			// Update the shared reactive state so all observers are notified.
			// For non-empty paths the underlying useLocalStorageValue setter
			// persists the wrapped value; for empty paths it stays in-memory.
			// Preserve any existing rev metadata on the entry.
			const current = entry.state.val as StoredDraft<unknown> | undefined
			entry.state.val = wrap(value, extractMeta(current))
			return
		}
		if (isLocalOnly(path)) return
		// External save without a live handle: preserve any persisted meta
		// so the staleness signal isn't lost just because the editor wasn't
		// open while we wrote.
		const existing = readPersisted<unknown>(localStorageKey(ws, itemKind, path))
		try {
			localStorage.setItem(
				localStorageKey(ws, itemKind, path),
				JSON.stringify(wrap(value, extractMeta(existing)))
			)
		} catch (e) {
			console.error('UserDraft.save: localStorage write failed', e)
		}
	},

	get<V = unknown>(
		itemKind: UserDraftItemKind,
		path: string,
		opts?: UserDraftOptions
	): V | undefined {
		const ws = resolveWorkspace(opts)
		const mk = mapKey(ws, itemKind, path)
		const entry = entries.get(mk)
		if (entry) {
			return unwrap(entry.state.val as StoredDraft<V> | undefined)
		}
		if (isLocalOnly(path)) return undefined
		return unwrap(readPersisted<V>(localStorageKey(ws, itemKind, path)))
	},

	/**
	 * Read the rev metadata for the entry. Returns an empty object if there
	 * is no entry. Useful for staleness checks before reading the draft.
	 */
	getMeta(itemKind: UserDraftItemKind, path: string, opts?: UserDraftOptions): UserDraftMeta {
		const ws = resolveWorkspace(opts)
		const mk = mapKey(ws, itemKind, path)
		const entry = entries.get(mk)
		if (entry) return extractMeta(entry.state.val as StoredDraft<unknown> | undefined)
		if (isLocalOnly(path)) return {}
		return extractMeta(readPersisted<unknown>(localStorageKey(ws, itemKind, path)))
	},

	/**
	 * Whether a draft currently exists for (workspace, itemKind, path).
	 * For non-empty paths this checks localStorage; for empty paths it
	 * checks the in-memory entry. Useful for distinguishing "first visit"
	 * from "returning visit with unsaved local changes".
	 */
	has(itemKind: UserDraftItemKind, path: string, opts?: UserDraftOptions): boolean {
		const ws = resolveWorkspace(opts)
		const mk = mapKey(ws, itemKind, path)
		const entry = entries.get(mk)
		if (entry) return entry.state.val !== undefined
		if (isLocalOnly(path)) return false
		return readPersisted(localStorageKey(ws, itemKind, path)) !== undefined
	},

	remove(itemKind: UserDraftItemKind, path: string, opts?: UserDraftOptions): void {
		const ws = resolveWorkspace(opts)
		try {
			localStorage.removeItem(localStorageKey(ws, itemKind, path))
		} catch (e) {
			console.error('UserDraft.remove: localStorage remove failed', e)
		}
	},

	use<V = unknown>(
		itemKind: UserDraftItemKind,
		path: string,
		opts?: UserDraftUseOptions<V>
	): UserDraftHandle<V> {
		const ws = resolveWorkspace(opts)
		const mk = mapKey(ws, itemKind, path)
		const wrappedDefault = wrap(opts?.defaultValue)

		let entry = entries.get(mk)
		if (!entry) {
			const state: DraftState<unknown> = isLocalOnly(path)
				? createInMemoryState<unknown>(wrappedDefault)
				: useLocalStorageValue<StoredDraft<unknown> | undefined>(
						localStorageKey(ws, itemKind, path),
						wrappedDefault,
						undefined,
						// The first value to flow into the handle (e.g. a backend load
						// in the editor route) is the baseline — only persist when the
						// user actually changes it afterwards.
						{ saveInitialValue: false }
					)
			entry = { count: 1, state }
			entries.set(mk, entry)
		} else {
			entry.count++
		}

		const sharedEntry = entry

		onDestroy(() => {
			const e = entries.get(mk)
			if (!e) return
			e.count--
			if (e.count <= 0) {
				entries.delete(mk)
			}
		})

		return {
			get draft(): V | undefined {
				return unwrap(sharedEntry.state.val as StoredDraft<V> | undefined)
			},
			set draft(value: V | undefined) {
				// Preserve existing rev metadata when the user just edits the
				// value (e.g. typing in the editor). useLocalStorageValue's
				// setter writes synchronously and removes the localStorage
				// entry when value is undefined.
				const current = sharedEntry.state.val as StoredDraft<V> | undefined
				sharedEntry.state.val = wrap(value, extractMeta(current))
			},
			get meta(): UserDraftMeta {
				return extractMeta(sharedEntry.state.val as StoredDraft<unknown> | undefined)
			},
			setDraftAndMeta(value: V | undefined, meta: UserDraftMeta): void {
				sharedEntry.state.val = wrap(value, meta)
			},
			setMeta(meta: UserDraftMeta): void {
				const current = sharedEntry.state.val as StoredDraft<V> | undefined
				if (current === undefined) return
				sharedEntry.state.val = wrap(current.value, meta)
			}
		}
	}
}

/** Test-only: clear all in-memory entries. */
export function __resetUserDraftForTesting(): void {
	entries.clear()
}
