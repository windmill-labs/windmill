import { BROWSER } from 'esm-env'
import { createSubscriber } from 'svelte/reactivity'
import type { UserDraftItemKind } from '$lib/gen'
import { scopedKey } from '$lib/userScopedStorage'
import { maskKey } from './modifiedItemsMask'

// Items the user edited by hand in a session's editor tabs, as mask keys
// (`${UserDraftItemKind}:${storagePath}`). Kept out of the chat record: turns and
// other tabs rewrite that record whole, so an entry written there outside a turn
// is lost to the next writer. For the same reason each entry is a localStorage key
// of its own: adding or removing one is a single setItem/removeItem, which no
// other tab's write can undo, as a read-modify-write of a shared list could.

const KEY_BASE = 'windmill-session-hand-edit'

function sessionPrefix(sessionId: string): string | undefined {
	const base = scopedKey(KEY_BASE)
	return base ? `${base}:${sessionId}:` : undefined
}

function storedKeys(prefix: string): string[] {
	const keys: string[] = []
	for (let i = 0; i < localStorage.length; i++) {
		const k = localStorage.key(i)
		if (k?.startsWith(prefix)) keys.push(k)
	}
	return keys
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

function write(sessionId: string, change: (prefix: string) => void): void {
	const prefix = sessionPrefix(sessionId)
	if (!BROWSER || !prefix) return
	try {
		change(prefix)
	} catch (e) {
		console.error('Could not store the session hand edits', e)
	}
	for (const update of localWriters) update()
}

/** The session's hand-edited items. Reactive. */
export function sessionHandEdits(sessionId: string): ReadonlySet<string> {
	const prefix = sessionPrefix(sessionId)
	if (!BROWSER || !prefix) return new Set()
	subscribe()
	try {
		return new Set(storedKeys(prefix).map((k) => k.slice(prefix.length)))
	} catch {
		return new Set()
	}
}

export function recordHandEdit(
	sessionId: string,
	itemKind: UserDraftItemKind,
	storagePath: string
): void {
	const prefix = sessionPrefix(sessionId)
	const k = prefix && prefix + maskKey(itemKind, storagePath)
	if (!BROWSER || !k || localStorage.getItem(k) !== null) return
	write(sessionId, () => localStorage.setItem(k, '1'))
}

/** For a discarded item, which would otherwise keep reading as this session's "Deployed" edit. */
export function removeHandEdit(
	sessionId: string,
	itemKind: UserDraftItemKind,
	storagePath: string
): void {
	write(sessionId, (prefix) => localStorage.removeItem(prefix + maskKey(itemKind, storagePath)))
}

/** For a draft-only item that deployed from its `draft_{uuid}` storage path to its chosen one. */
export function renameHandEdit(
	sessionId: string,
	itemKind: UserDraftItemKind,
	fromPath: string,
	toPath: string
): void {
	if (fromPath === toPath) return
	write(sessionId, (prefix) => {
		const from = prefix + maskKey(itemKind, fromPath)
		if (localStorage.getItem(from) === null) return
		localStorage.setItem(prefix + maskKey(itemKind, toPath), '1')
		localStorage.removeItem(from)
	})
}

export function deleteSessionHandEdits(sessionId: string): void {
	write(sessionId, (prefix) => {
		for (const k of storedKeys(prefix)) localStorage.removeItem(k)
	})
}
