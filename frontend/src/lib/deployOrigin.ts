import { OpenAPI } from '$lib/gen'

/** Header that tells the fork tally a write was applied to the workspace rather
 * than authored in it. Mirrors `DEPLOY_ORIGIN_HEADER` in `windmill-common`. */
const DEPLOY_ORIGIN_HEADER = 'X-Windmill-Deploy-Origin'

let syncDepth = 0

/**
 * Run `apply` with every request it makes marked as applied by a sync.
 *
 * The mark is per-tab, not per-request: a request the user happens to fire while
 * the apply is in flight is marked too. That only ever makes the tally more
 * conservative — a sync-origin write is never taken as evidence that this
 * workspace dropped an item — so the imprecision costs an offer, never data.
 */
export async function asSyncDeploy<T>(apply: () => Promise<T>): Promise<T> {
	syncDepth++
	try {
		return await apply()
	} finally {
		syncDepth--
	}
}

// Registered once for the session; a no-op outside `asSyncDeploy`.
OpenAPI.interceptors.request.use((options) => {
	if (syncDepth > 0) {
		const headers = new Headers(options.headers)
		headers.set(DEPLOY_ORIGIN_HEADER, 'sync')
		options.headers = headers
	}
	return options
})
