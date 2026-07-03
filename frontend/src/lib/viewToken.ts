import { OpenAPI } from '$lib/gen'

/**
 * Share-read-link support. When viewing a run via a share link
 * (`/run/{id}?view_token=...`), the token grants the current authenticated member
 * read access to that job and its flow subtree on the backend.
 *
 * The token is attached to every generated-client request via the `X-View-Token`
 * header (registered once below) so we don't have to thread it through every
 * `JobService` call. `EventSource`/SSE can't set headers, so those URLs read
 * `getViewToken()` and append it as a `view_token` query param instead.
 */
let currentViewToken: string | undefined = undefined

export function setViewToken(token: string | undefined): void {
	currentViewToken = token || undefined
}

export function getViewToken(): string | undefined {
	return currentViewToken
}

/**
 * Append the current view token as a `view_token` query param to a URL/path.
 * Used for download links (plain `<a href>` and `downloadViaClient`), which don't
 * go through the request interceptor that adds the `X-View-Token` header.
 * Returns the url unchanged when no share link is active.
 */
export function appendViewToken(url: string): string {
	if (!currentViewToken) return url
	const sep = url.includes('?') ? '&' : '?'
	return `${url}${sep}view_token=${encodeURIComponent(currentViewToken)}`
}

// Register the request interceptor exactly once. It is a no-op unless a view token
// is currently set, so it is safe to keep installed for the whole session.
OpenAPI.interceptors.request.use((options) => {
	if (currentViewToken) {
		const headers = new Headers(options.headers)
		headers.set('X-View-Token', currentViewToken)
		options.headers = headers
	}
	return options
})
