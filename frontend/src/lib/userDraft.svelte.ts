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
	| 'schedule_schedule'
	| 'schedule_webhook'
	| 'schedule_default_email'
	| 'schedule_email'
	| 'schedule_http'
	| 'schedule_websocket'
	| 'schedule_postgres'
	| 'schedule_kafka'
	| 'schedule_nats'
	| 'schedule_mqtt'
	| 'schedule_sqs'
	| 'schedule_gcp'
	| 'schedule_azure'
	| 'schedule_poll'
	| 'schedule_cli'
	| 'schedule_nextcloud'
	| 'schedule_google'
	| 'schedule_github'

export type UserDraftOptions = {
	workspace?: string
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
			entry.state.val = value
			return
		}
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
		try {
			const raw = localStorage.getItem(localStorageKey(ws, itemKind, path))
			if (raw == null || raw === 'undefined') return undefined
			return JSON.parse(raw) as V
		} catch (e) {
			console.error('UserDraft.get: localStorage read failed', e)
			return undefined
		}
	},

	remove(itemKind: UserDraftItemKind, path: string, opts?: UserDraftOptions): void {
		const ws = resolveWorkspace(opts)
		const mk = mapKey(ws, itemKind, path)
		const lk = localStorageKey(ws, itemKind, path)
		const entry = entries.get(mk)
		if (entry) {
			// Notify observers via the shared state setter.
			entry.state.val = undefined
		}
		try {
			// Always remove (the setter above would persist "undefined" as a string).
			localStorage.removeItem(lk)
		} catch (e) {
			console.error('UserDraft.remove: localStorage remove failed', e)
		}
	},

	use<V = unknown>(
		itemKind: UserDraftItemKind,
		path: string,
		opts?: UserDraftOptions
	): UserDraftHandle<V> {
		const ws = resolveWorkspace(opts)
		const mk = mapKey(ws, itemKind, path)
		const lk = localStorageKey(ws, itemKind, path)

		let entry = entries.get(mk)
		if (!entry) {
			entry = { count: 1, state: useLocalStorageValue<unknown>(lk, undefined) }
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
				sharedEntry.state.val = value
				if (value === undefined) {
					try {
						localStorage.removeItem(lk)
					} catch (e) {
						console.error('UserDraft.use: localStorage remove failed', e)
					}
				}
			}
		}
	}
}

/** Test-only: clear all in-memory entries. */
export function __resetUserDraftForTesting(): void {
	entries.clear()
}
