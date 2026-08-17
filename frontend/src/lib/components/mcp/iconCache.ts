import { providerKey } from './providerIcon'

/**
 * Remembers which provider a connection points at, so the icons are there on
 * the first paint of every later visit.
 *
 * The url lives in the resource value, which `listResource` deliberately does
 * not return, so drawing an icon otherwise costs one read per row on every
 * open. `edited_at` comes back with the list, so a row that has not been edited
 * since it was cached needs no read at all.
 */
type Entry = { key: string | null; editedAt?: string }

const STORE_KEY = 'mcp_provider_icons'

function read(): Record<string, Record<string, Entry>> {
	try {
		return JSON.parse(localStorage.getItem(STORE_KEY) ?? '{}')
	} catch {
		return {}
	}
}

export function cachedProviderKey(
	workspace: string,
	path: string,
	editedAt?: string
): string | null | undefined {
	const entry = read()[workspace]?.[path]
	if (!entry) return undefined
	// A path can be reconnected to a different server, and then the icon would be
	// the previous provider's.
	return entry.editedAt === editedAt ? entry.key : undefined
}

export function rememberProviderKey(
	workspace: string,
	path: string,
	url: unknown,
	editedAt?: string
): string | null {
	const key = providerKey(url) ?? null
	const store = read()
	store[workspace] = { ...(store[workspace] ?? {}), [path]: { key, editedAt } }
	try {
		localStorage.setItem(STORE_KEY, JSON.stringify(store))
	} catch {}
	return key
}

export function forgetProviderKey(workspace: string, path: string) {
	const store = read()
	if (!store[workspace]?.[path]) return
	delete store[workspace][path]
	try {
		localStorage.setItem(STORE_KEY, JSON.stringify(store))
	} catch {}
}
