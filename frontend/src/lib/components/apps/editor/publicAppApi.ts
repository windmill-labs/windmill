// The public app routes (`/public/...`, `/a/...`) render a raw app as little more than its
// bundle's iframe, so they must not statically import the generated client, `$lib/stores`
// or `$lib/utils`: each drags in most of the app shell. `vite.config.js`
// (`assertLeanPublicAppRoutes`) fails the build if they do. Anything heavier is imported
// lazily by the branch that needs it.
import { OpenAPI } from '$lib/gen/core/OpenAPI'
import type { AppWithLastVersion, User } from '$lib/gen/types.gen'
import type { UserExt } from '$lib/stores'
import { mapUserToUserExt } from '$lib/userExt'

export type PublicAppValue = AppWithLastVersion & { value: any; workspace_id?: string }

declare global {
	interface Window {
		/** Responses `app.html` requested before any module loaded, keyed by URL. */
		__wmPublicAppPrefetch?: Record<string, Promise<Response>>
	}
}

/** Carries `status` like the generated client's `ApiError`, which callers branch on. */
export class PublicApiError extends Error {
	constructor(public status: number) {
		super(`Request failed with status ${status}`)
	}
}

/** Prefetched responses answer for the credential the page loaded with; call this when it
 * changes (a sign-in or logout on the page) so later reads go to the network. */
export function dropPrefetched(): void {
	delete window.__wmPublicAppPrefetch
}

function takePrefetched(url: string): Promise<Response> | undefined {
	const prefetched = window.__wmPublicAppPrefetch
	const res = prefetched?.[url]
	if (res) delete prefetched![url]
	return res
}

/** GET an API path with the page credential: the bearer token in `OpenAPI.TOKEN` when the
 * share link carries one, the session cookie otherwise. A prefetched response is only
 * taken in the cookie case, which is the only credential `app.html` sends. */
export async function publicGet<T>(path: string): Promise<T> {
	const url = `${OpenAPI.BASE}${path}`
	const token = typeof OpenAPI.TOKEN === 'string' && OpenAPI.TOKEN ? OpenAPI.TOKEN : undefined
	const res = await ((!token && takePrefetched(url)) ||
		fetch(url, {
			// As the generated client does: the opaque viewer frame is cross-origin to the
			// API, whose `Access-Control-Allow-Origin: *` refuses a credentialed request.
			credentials: OpenAPI.WITH_CREDENTIALS ? OpenAPI.CREDENTIALS : undefined,
			headers: token ? { Authorization: `Bearer ${token}` } : {}
		}))
	if (!res.ok) throw new PublicApiError(res.status)
	return (res.headers.get('content-type')?.includes('json') ? res.json() : res.text()) as T
}

export async function getUserExtOrUndefined(workspace: string): Promise<UserExt | undefined> {
	try {
		return mapUserToUserExt(
			await publicGet<User>(`/w/${encodeURI(workspace)}/users/whoami`),
			workspace
		)
	} catch {
		return undefined
	}
}

/** Only fades the "Powered by" badge, so a failure is not worth surfacing. */
export async function getLicenseOrUndefined(): Promise<string | undefined> {
	try {
		return (await publicGet<string>('/ee_license')) || undefined
	} catch {
		return undefined
	}
}

let appCss: Promise<unknown> | undefined
/** The app-wide stylesheet, which these routes need only off the raw-app path: sign-in,
 * errors, skeletons and low-code apps. A raw app's bundle brings its own styles. */
export function loadAppCss(): Promise<unknown> {
	return (appCss ??= import('$lib/assets/app.css'))
}
