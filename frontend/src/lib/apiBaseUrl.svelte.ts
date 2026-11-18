import { fromStore } from 'svelte/store'
import { instanceUi } from '$lib/instanceUi'

const ui = fromStore(instanceUi)

/**
 * The groups of external-client URLs the `api_base_url` setting can be narrowed to, through
 * `api_base_url_applies_to`. Mirrors `API_BASE_URL_SURFACES` on the server.
 */
export const API_BASE_URL_SURFACES = [
	{ value: 'webhooks', label: 'Webhooks and HTTP routes' },
	{ value: 'push_triggers', label: 'Push triggers (GCP, Azure)' },
	{ value: 'cli', label: 'CLI and API examples' },
	{ value: 'mcp', label: 'MCP server' }
] as const

export type ApiBaseUrlSurface = (typeof API_BASE_URL_SURFACES)[number]['value']

/**
 * Base of every URL shown for an external client to call (webhooks, HTTP routes, push
 * trigger endpoints, the CLI remote, the MCP server): the `api_base_url` instance setting
 * when it applies to `surface`, else the origin being browsed. Append `/api/...` to it.
 *
 * Not for URLs the editor itself calls or opens (captures, previews, OAuth popups, links
 * to pages): those must stay on the browsing origin, where the session cookie lives.
 *
 * Reactive: the setting lands after page load, so read it inside a `$derived` or the
 * template rather than caching the result.
 */
export function apiBaseUrl(surface: ApiBaseUrlSurface): string {
	const setting = ui.current?.api_base_url
	const configured = typeof setting === 'string' ? setting.trim() : ''
	const appliesTo = ui.current?.api_base_url_applies_to
	// Unset means every surface; the server reads it the same way for MCP.
	const applies = !Array.isArray(appliesTo) || appliesTo.includes(surface)
	return (applies && configured) || window.location.origin
}
