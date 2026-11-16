import { resource } from 'runed'
import {
	cacheIdentity,
	cacheKeys,
	loadEntry,
	peekEntry,
	saveEntry,
	type CacheScope
} from '$lib/persistedCache.svelte'
import { readSessionHint, sameSessionSince } from '$lib/lastSession'

export interface PersistOptions<A> {
	/** Names the response; part of every key. */
	name: string
	/** Bump whenever the stored value's shape changes: entries of another version read as none. */
	version: number
	/** What the response for `args` is filed under; undefined keeps it out of the cache. */
	scope: (args: A) => CacheScope | undefined
}

type Keys = { scope: string; key: string }
export type Tagged<T> = { keys: Keys | undefined; value: T | undefined }

/**
 * What to show for `keys`: the response fetched for them, else the cached one, else a response
 * of the same scope fetched for other arguments (a filter change keeps the list up while the
 * new one loads). Never a response of another user or workspace.
 */
export function pickShown<T>(
	fresh: Tagged<T> | undefined,
	keys: Keys | undefined,
	cached: T | undefined
): T | undefined {
	if (!keys) return fresh?.keys === undefined ? fresh?.value : undefined
	if (fresh?.keys?.key === keys.key && fresh.value !== undefined) return fresh.value
	if (cached !== undefined) return cached
	return fresh?.keys?.scope === keys.scope ? fresh.value : undefined
}

function keysFor<A>(opts: PersistOptions<A>, args: A): Keys | undefined {
	const identity = cacheIdentity()
	const scope = identity ? opts.scope(args) : undefined
	return identity && scope ? cacheKeys(identity, opts.name, scope) : undefined
}

/**
 * runed's `resource`, painting the last response stored for the same user, workspace and
 * arguments until its own request returns, then replacing it in place. A fetch that throws
 * keeps what is shown, cached or fetched, rather than clearing it.
 */
export function persistedResource<A, T>(
	source: () => A,
	fetcher: (args: A) => Promise<T | undefined>,
	opts: PersistOptions<A>
) {
	// runed's `resource` neither orders overlapping requests nor ties `loading` to the newest:
	// an older response landing last would replace the newer one, and the first to settle would
	// end `loading` while a later request is still in flight. Only the latest request does either.
	let latest = 0
	let inFlight = $state(false)
	let error = $state<unknown>(undefined)
	// The identity is a source too: a response is tagged with the keys of its request, and once
	// the identity changes no response tagged before matches the keys shown, so without a new
	// request nothing would be shown until the arguments next change.
	const sourceWithIdentity = () => ({ args: source(), identity: cacheIdentity() })
	const res = resource(sourceWithIdentity, async ({ args }): Promise<Tagged<T>> => {
		const id = ++latest
		inFlight = true
		try {
			const keys = keysFor(opts, args)
			if (keys) void loadEntry(keys.key, opts.version)
			const sentWith = readSessionHint()
			const value = await fetcher(args)
			if (keys && value !== undefined && sameSessionSince(sentWith))
				saveEntry(keys.key, opts.version, value)
			// An AbortError is how runed is told to keep its value without recording an error.
			if (id !== latest) throw new DOMException('superseded', 'AbortError')
			error = undefined
			return { keys, value }
		} catch (e) {
			if (id === latest) error = e
			throw e
		} finally {
			if (id === latest) inFlight = false
		}
	})
	const keys = $derived(keysFor(opts, source()))
	const shown = $derived(
		pickShown(res.current, keys, keys ? peekEntry<T>(keys.key, opts.version) : undefined)
	)
	return {
		get current() {
			return shown
		},
		get loading() {
			return inFlight
		},
		/** Why the latest request failed; cleared when one succeeds. */
		get error() {
			return error
		},
		refetch() {
			return res.refetch()
		}
	}
}

/**
 * The same cache for a call site that fetches imperatively: `peek` what is stored for a scope,
 * `load` it from disk when memory has none yet, `save` a response.
 */
export function persistedValue<T>(name: string, version: number) {
	const keyOf = (scope: CacheScope) => {
		const identity = cacheIdentity()
		return identity ? cacheKeys(identity, name, scope).key : undefined
	}
	return {
		peek(scope: CacheScope): T | undefined {
			const key = keyOf(scope)
			return key ? peekEntry<T>(key, version) : undefined
		},
		async load(scope: CacheScope): Promise<T | undefined> {
			const key = keyOf(scope)
			return key ? loadEntry<T>(key, version) : undefined
		},
		/** `sentWith`: the session hint when the request was sent (`readSessionHint`). */
		save(scope: CacheScope, value: T, sentWith: string | undefined) {
			const key = keyOf(scope)
			if (key && sameSessionSince(sentWith)) saveEntry(key, version, value)
		}
	}
}
