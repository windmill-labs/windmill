export type Tagged<T> = { key: string; seq: number; value: T }

/**
 * Stamps each result with the scope it describes and the order its request was issued in.
 * `resource` orders nothing: it assigns `current` unconditionally on resolve, and cancels
 * through an `AbortSignal` the generated client cannot consume. The scope alone cannot
 * order two requests for one scope, so the issue order travels alongside it.
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
 * Holds the newest value fetched for `key`; a late answer for a scope we left, or one
 * overtaken for this scope, neither publishes nor erases. A failed refresh leaves the
 * last successful value standing, which is why `loading` cannot gate this: it is true
 * throughout a re-read whose held value is still the right one to show.
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
