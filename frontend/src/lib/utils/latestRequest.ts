/**
 * Lets a reply tell whether a later request superseded it, so that whichever reply arrives last
 * cannot overwrite the newer one's results. Take the check before the request and drop the reply
 * when it no longer holds: a guard on the workspace alone misses filter changes and round trips.
 */
export function latestRequest() {
	let generation = 0
	return {
		/** Starts a request: the check fails once a later one starts. */
		start(): () => boolean {
			const g = ++generation
			return () => g === generation
		},
		/** The same check, without superseding the request in flight. */
		peek(): () => boolean {
			const g = generation
			return () => g === generation
		}
	}
}
