import { BROWSER } from 'esm-env'
import { userStore } from '$lib/stores'
import { getLocalSetting, storeLocalSetting } from '$lib/utils'

// Browser-persisted AI-session state (the `/sessions` list, unread markers,
// chat-history IndexedDB, autonomy mode) must be scoped to the logged-in
// instance user so a shared browser never leaks one user's data to the next.
//
// The scoping identity is the instance user **email** — the only identity
// stable across a fork family (a single session deliberately spans multiple
// workspace_ids; `username` varies per workspace, `email` does not).
//
// The email arrives asynchronously (userStore is populated after the layout's
// getUserExt), while several consumers initialise eagerly at module import.
// This module owns the single subscription and lets each surface register a
// hydrate+migrate callback that fires once on registration (covering late
// registrants whose email already resolved) and again on every email change.

type UserChangeCallback = (email: string | undefined, prevEmail: string | undefined) => void

let currentEmail: string | undefined = undefined
const callbacks = new Set<UserChangeCallback>()

if (BROWSER) {
	// userStore.subscribe fires synchronously with the current value (undefined
	// at import time) and again whenever the user resolves / changes. Logout is
	// a full page reload, and email is constant across workspace switches, so in
	// practice the only transition is the initial undefined → email hydration.
	userStore.subscribe((u) => {
		const next = u?.email
		if (next === currentEmail) return
		const prev = currentEmail
		currentEmail = next
		for (const cb of callbacks) {
			try {
				cb(next, prev)
			} catch (e) {
				console.error('userScopedStorage: onUserChange callback failed', e)
			}
		}
	})
}

// Current logged-in user's email, or undefined before it resolves / when
// logged out.
export function getCurrentUserEmail(): string | undefined {
	return currentEmail
}

// Namespace a base storage key (localStorage key or IndexedDB name) by the
// current user's email. Returns undefined when no user is known — callers must
// treat that as "do not read/write" so we never touch a browser-global key.
export function scopedKey(base: string): string | undefined {
	if (!currentEmail) return undefined
	return scopedKeyFor(base, currentEmail)
}

// The key a base name has for a given user, for work that captured its user up front and
// must not follow an in-place account switch (the session backup flush).
export function scopedKeyFor(base: string, email: string): string {
	return `${base}::${email}`
}

// The email a scoped key or database name was built for, so a write that landed in a
// store can name the user it belongs to even after the current user changed.
export function emailOfScopedKey(base: string, key: string): string | undefined {
	return key.startsWith(`${base}::`) ? key.slice(base.length + 2) : undefined
}

// Register a callback invoked whenever the scoping email changes. Fired once
// immediately with the current email (prevEmail undefined) so a surface that
// registers after the email already resolved still hydrates.
export function onUserChange(cb: UserChangeCallback): void {
	callbacks.add(cb)
	try {
		cb(currentEmail, undefined)
	} catch (e) {
		console.error('userScopedStorage: initial onUserChange callback failed', e)
	}
}

// One-shot claim of pre-namespacing data: if the scoped key is empty and a
// legacy un-namespaced key exists, move it under the scoped key and delete the
// legacy copy. The first user to log in on a previously single-user browser
// keeps their data; subsequent users start clean.
export function migrateLegacyLocalStorage(legacyKey: string, targetKey: string | undefined): void {
	if (!BROWSER || !targetKey) return
	try {
		if (getLocalSetting(targetKey) != null) return
		const legacy = getLocalSetting(legacyKey)
		if (legacy == null) return
		storeLocalSetting(targetKey, legacy)
		storeLocalSetting(legacyKey, undefined)
	} catch (e) {
		console.error('userScopedStorage: legacy localStorage migration failed', e)
	}
}
