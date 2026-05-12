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

type DraftState<V> = { val: V | undefined }

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

function createInMemoryState<T>(defaultValue: T | undefined): DraftState<T> {
	let s = $state<T | undefined>(defaultValue)
	return {
		get val(): T | undefined {
			return s
		},
		set val(newVal: T | undefined) {
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
}

export const UserDraft = {
	save<V>(itemKind: UserDraftItemKind, path: string, value: V, opts?: UserDraftOptions): void {
		const ws = resolveWorkspace(opts)
		const mk = mapKey(ws, itemKind, path)
		const entry = entries.get(mk)
		if (entry) {
			// Update the shared reactive state so all observers are notified.
			// For non-empty paths the underlying useLocalStorageValue setter
			// persists; for empty paths the in-memory state stays in-memory.
			entry.state.val = value
			return
		}
		if (isLocalOnly(path)) return
		try {
			localStorage.setItem(localStorageKey(ws, itemKind, path), JSON.stringify(value))
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
			return entry.state.val as V | undefined
		}
		if (isLocalOnly(path)) return undefined
		try {
			const raw = localStorage.getItem(localStorageKey(ws, itemKind, path))
			if (raw == null || raw === 'undefined') return undefined
			return JSON.parse(raw) as V
		} catch (e) {
			console.error('UserDraft.get: localStorage read failed', e)
			return undefined
		}
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
		try {
			const raw = localStorage.getItem(localStorageKey(ws, itemKind, path))
			return raw != null && raw !== 'undefined'
		} catch {
			return false
		}
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
		const defaultValue = opts?.defaultValue

		let entry = entries.get(mk)
		if (!entry) {
			const state: DraftState<unknown> = isLocalOnly(path)
				? createInMemoryState<unknown>(defaultValue)
				: useLocalStorageValue<unknown>(
						localStorageKey(ws, itemKind, path),
						defaultValue,
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
				return sharedEntry.state.val as V | undefined
			},
			set draft(value: V | undefined) {
				// useLocalStorageValue's setter writes synchronously and
				// removes the localStorage entry when value is undefined.
				sharedEntry.state.val = value
			}
		}
	}
}

/** Test-only: clear all in-memory entries. */
export function __resetUserDraftForTesting(): void {
	entries.clear()
}
