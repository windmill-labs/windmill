const LAST_RELOAD_KEY = 'wm_stale_chunk_reload_at'
const MIN_INTERVAL_MS = 60_000

/** The rejection message of a dynamic import whose chunk could not be fetched. */
export function isChunkLoadError(message: string): boolean {
	return (
		message.startsWith('Failed to fetch dynamically imported') || // Chromium
		message.startsWith('error loading dynamically imported module') || // Firefox
		message.startsWith('Importing a module script failed') // Safari
	)
}

/**
 * A tab left open across a deploy asks for chunk names from the previous build, which are
 * gone: reloading picks up the current build. At most once a minute, so a chunk missing
 * from the current build too does not reload in a loop. Returns whether it reloaded.
 */
export function reloadForStaleChunk(): boolean {
	try {
		const last = Number(sessionStorage.getItem(LAST_RELOAD_KEY) ?? 0)
		if (Date.now() - last < MIN_INTERVAL_MS) return false
		sessionStorage.setItem(LAST_RELOAD_KEY, String(Date.now()))
	} catch {
		return false
	}
	location.reload()
	return true
}
