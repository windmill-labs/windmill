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
// `host` backs the favicon fallback for a server Windmill ships no icon for. It is
// cached alongside the key for the same reason: the url is only in the resource
// value, which the list endpoint strips.
type Entry = { key: string | null; editedAt?: string; host?: string }

// Versioned: entries written before `host` existed hold a key (often `null`, the
// no-shipped-icon case the favicon fallback is for) and read as a cache hit, so they
// would never learn a host and never draw a favicon. Bumping starts them over, at one
// resource read each.
const STORE_KEY = 'mcp_provider_icons_v2'

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

/**
 * The stored mark for a path, ignoring `editedAt`. A transcript row marks a call
 * that already happened, so the provider from before a reconnect is still the one
 * to draw — and unlike `readOnlyHint`, nothing acts on it.
 */
export function cachedProviderMark(
	workspace: string,
	path: string
): { key: string | null; host?: string } | undefined {
	const entry = read()[workspace]?.[path]
	return entry ? { key: entry.key, host: entry.host } : undefined
}

/** Suffixes that only ever name something inside a network, so a favicon lookup for
 * one would disclose an internal endpoint and could not succeed anyway. */
const PRIVATE_HOST_SUFFIXES = [
	'.local',
	'.localhost',
	'.internal',
	'.intranet',
	'.lan',
	'.corp',
	'.home',
	'.localdomain'
]

/**
 * The url's host, when it has one worth drawing a favicon for.
 *
 * Fetching a favicon tells the favicon service which host was asked about, so this
 * withholds what it can recognise as private. It cannot recognise all of it: a public
 * domain used internally (`mcp.internal.example.com`) is indistinguishable from any
 * other, so a self-hosted instance still discloses that hostname when its server has
 * no shipped icon.
 */
export function providerHost(url: unknown): string | undefined {
	if (typeof url !== 'string') return undefined
	try {
		const hostname = new URL(url).hostname.toLowerCase()
		// A loopback, a bare address, or an intranet single-label name has no favicon
		// to fetch, and asking would disclose it to the favicon service for nothing.
		if (!hostname.includes('.') || /^[\d.]+$/.test(hostname)) return undefined
		if (PRIVATE_HOST_SUFFIXES.some((suffix) => hostname.endsWith(suffix))) return undefined
		return hostname
	} catch {
		return undefined
	}
}

export function cachedProviderHost(
	workspace: string,
	path: string,
	editedAt?: string
): string | undefined {
	const entry = read()[workspace]?.[path]
	return entry?.editedAt === editedAt ? entry?.host : undefined
}

export function rememberProviderKey(
	workspace: string,
	path: string,
	url: unknown,
	editedAt?: string
): string | null {
	const key = providerKey(url) ?? null
	const store = read()
	store[workspace] = {
		...(store[workspace] ?? {}),
		[path]: { key, editedAt, host: providerHost(url) }
	}
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
