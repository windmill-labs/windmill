import { fromStore } from 'svelte/store'
import { base } from '$lib/base'
import { instanceUi } from '$lib/instanceUi'

const ui = fromStore(instanceUi)

/** The `api_base_url` instance setting when it holds a URL, else `fallback`. */
export function resolveApiBaseUrl(setting: unknown, fallback: string): string {
	const configured = typeof setting === 'string' ? setting.trim() : ''
	return configured || fallback
}

/**
 * Base of every URL shown for an external client to call (webhooks, HTTP routes, push
 * trigger endpoints, the CLI remote): the `api_base_url` instance setting, else the
 * origin being browsed. Append `/api/...` to it.
 *
 * Not for URLs the editor itself calls or opens (captures, previews, OAuth popups, links
 * to pages): those must stay on the browsing origin, where the session cookie lives.
 *
 * Reactive: the setting lands after page load, so read it inside a `$derived` or the
 * template rather than caching the result.
 */
export function apiBaseUrl(): string {
	return resolveApiBaseUrl(ui.current?.api_base_url, `${window.location.origin}${base}`)
}
