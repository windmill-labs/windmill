export type Tagged<T> = { key: string; seq: number; value: T }

/**
 * Wraps a fetcher so each result carries the scope it describes and the order its
 * request was issued in.
 *
 * `resource` does not order responses: it cancels through an `AbortSignal` the generated
 * client cannot consume, and assigns `current` unconditionally once a fetch resolves.
 * The scope alone cannot order two requests for the *same* scope — a refetch landing on
 * an in-flight load, or a second invalidation — so the issue order travels with them.
 */
export function tagged<K extends string, V>(
	fetch: (key: K) => Promise<V>
): (key: K) => Promise<Tagged<V>> {
	let issued = 0
	return async (key: K) => {
		const seq = ++issued
		return { key, seq, value: await fetch(key) }
	}
}

/**
 * Holds the newest fetched value that describes `key`, ignoring any other — a late
 * answer for a scope we have left, and an answer overtaken by a newer one for this
 * scope, neither publish nor erase what is on screen.
 *
 * `loading` cannot do this job: it is also true during a `refetch()`, when the held
 * value is still the right one, so gating on it blanks the display on every re-read.
 */
export function scopedValue<T>() {
	let held: Tagged<T> | undefined = undefined
	return (key: string | undefined, fetched: Tagged<T> | undefined) => {
		if (fetched && fetched.key === key && (held?.key !== key || fetched.seq > held.seq)) {
			held = fetched
		}
		return held && held.key === key ? held.value : undefined
	}
}
