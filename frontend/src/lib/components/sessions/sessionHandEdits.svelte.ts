import { BROWSER } from 'esm-env'
import { createSubscriber } from 'svelte/reactivity'
import type { UserDraftItemKind } from '$lib/gen'
import { scopedKey } from '$lib/userScopedStorage'
import { maskKey } from './modifiedItemsMask'

// Items the user edited by hand in a session's editor tabs, as mask keys
// (`${UserDraftItemKind}:${storagePath}`). Kept out of the chat record: turns and
// other tabs rewrite that record whole, so an entry written there outside a turn
// is lost to the next writer. Each write here re-reads the stored list and
// changes only its own key, so tabs never undo one another's entries.

const KEY_BASE = 'windmill-session-hand-edits'

function storageKey(sessionId: string): string | undefined {
	const base = scopedKey(KEY_BASE)
	return base ? `${base}:${sessionId}` : undefined
}

function readStored(sessionId: string): string[] {
	const key = storageKey(sessionId)
	if (!BROWSER || !key) return []
	try {
		const parsed: unknown = JSON.parse(localStorage.getItem(key) ?? '[]')
		return Array.isArray(parsed) ? parsed.filter((k): k is string => typeof k === 'string') : []
	} catch {
		return []
	}
}

const localWriters = new Set<() => void>()

// Local writes and other tabs' writes (the `storage` event) both re-run readers.
const subscribe = createSubscriber((update) => {
	const onStorage = (e: StorageEvent) => {
		if (e.key === null || e.key.startsWith(KEY_BASE)) update()
	}
	window.addEventListener('storage', onStorage)
	localWriters.add(update)
	return () => {
		window.removeEventListener('storage', onStorage)
		localWriters.delete(update)
	}
})

function write(sessionId: string, change: (keys: Set<string>) => boolean): void {
	const key = storageKey(sessionId)
	if (!BROWSER || !key) return
	const keys = new Set(readStored(sessionId))
	if (!change(keys)) return
	try {
		if (keys.size > 0) localStorage.setItem(key, JSON.stringify([...keys]))
		else localStorage.removeItem(key)
	} catch (e) {
		console.error('Could not store the session hand edits', e)
	}
	for (const update of localWriters) update()
}

/** The session's hand-edited items. Reactive. */
export function sessionHandEdits(sessionId: string): ReadonlySet<string> {
	if (BROWSER) subscribe()
	return new Set(readStored(sessionId))
}

export function recordHandEdit(
	sessionId: string,
	itemKind: UserDraftItemKind,
	storagePath: string
): void {
	const k = maskKey(itemKind, storagePath)
	write(sessionId, (keys) => !keys.has(k) && !!keys.add(k))
}

/** For a discarded item, which would otherwise keep reading as this session's "Deployed" edit. */
export function removeHandEdit(
	sessionId: string,
	itemKind: UserDraftItemKind,
	storagePath: string
): void {
	write(sessionId, (keys) => keys.delete(maskKey(itemKind, storagePath)))
}

/** For a draft-only item that deployed from its `draft_{uuid}` storage path to its chosen one. */
export function renameHandEdit(
	sessionId: string,
	itemKind: UserDraftItemKind,
	fromPath: string,
	toPath: string
): void {
	if (fromPath === toPath) return
	write(sessionId, (keys) => {
		if (!keys.delete(maskKey(itemKind, fromPath))) return false
		keys.add(maskKey(itemKind, toPath))
		return true
	})
}

export function deleteSessionHandEdits(sessionId: string): void {
	write(sessionId, (keys) => {
		const had = keys.size > 0
		keys.clear()
		return had
	})
}
