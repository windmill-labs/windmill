import { base } from '$lib/base'
import type { WorkspaceItemKind } from '$lib/components/workspacePicker'

// The paths a preview location can point at, and the base handling around them. Kept apart
// from `previewRouter`, which reads a location's *view* from each page's filter schema and
// through those reaches every trigger's save utils: a drawer that needs nothing but a page
// path is mounted inside script and flow editors, which must not pull that in.

// In-app paths for the deep-linkable preview pages the AI chat can open.
export const RUNS_PATH = '/runs'
export const SCHEDULES_PATH = '/schedules'
export const VARIABLES_PATH = '/variables'
export const RESOURCES_PATH = '/resources'
export const ASSETS_PATH = '/assets'
export const AUDIT_LOGS_PATH = '/audit_logs'
export const WORKSPACE_SETTINGS_PATH = '/workspace_settings'
export const FOLDERS_PATH = '/folders'
export const GROUPS_PATH = '/groups'

// Trigger list pages, by kind. Deliberately kept out of PREVIEW_PAGES (the curated
// breadcrumb picker) but shared here so open_page can route to them and the preview tab
// can label them. `ee` kinds require an enterprise license. Each supports `#<path>` to
// open a specific trigger, like Schedules.
export type TriggerKind =
	| 'http'
	| 'websocket'
	| 'postgres'
	| 'kafka'
	| 'nats'
	| 'sqs'
	| 'gcp'
	| 'azure'
	| 'mqtt'
	| 'amqp'
	| 'email'

export const TRIGGER_PAGES: Record<TriggerKind, { path: string; label: string; ee?: boolean }> = {
	http: { path: '/routes', label: 'HTTP routes' },
	websocket: { path: '/websocket_triggers', label: 'WebSocket triggers' },
	postgres: { path: '/postgres_triggers', label: 'Postgres triggers' },
	kafka: { path: '/kafka_triggers', label: 'Kafka triggers', ee: true },
	nats: { path: '/nats_triggers', label: 'NATS triggers', ee: true },
	sqs: { path: '/sqs_triggers', label: 'SQS triggers', ee: true },
	gcp: { path: '/gcp_triggers', label: 'GCP Pub/Sub triggers', ee: true },
	azure: { path: '/azure_triggers', label: 'Azure Event Grid triggers', ee: true },
	mqtt: { path: '/mqtt_triggers', label: 'MQTT triggers' },
	amqp: { path: '/amqp_triggers', label: 'AMQP triggers' },
	email: { path: '/email_triggers', label: 'Email triggers' }
}

/** A workspace item edited from its list page rather than at an editor route: a variable,
 * resource, schedule or trigger. A session hosts each in a tab of its own. */
export type PageItemRef =
	| { kind: 'variable' | 'resource' | 'schedule'; path: string }
	| { kind: 'trigger'; triggerKind: TriggerKind; path: string }

/** How a list page addresses a row in its hash. Resources route theirs through an extra
 * segment; every other page names the path directly. */
export const drawerHashFor = (pagePath: string, itemPath: string) =>
	pagePath === RESOURCES_PATH ? `/resource/${itemPath}` : itemPath

/** The full page a page item is edited on: its list page, with the row's drawer open. */
export function pageItemPageHref(ref: PageItemRef): string {
	const listPath = pageItemListPath(ref)
	return `${pageHref(listPath)}#${drawerHashFor(listPath, ref.path)}`
}

/** The list page a page item is edited from. */
export function pageItemListPath(ref: PageItemRef): string {
	switch (ref.kind) {
		case 'variable':
			return VARIABLES_PATH
		case 'resource':
			return RESOURCES_PATH
		case 'schedule':
			return SCHEDULES_PATH
		case 'trigger':
			return TRIGGER_PAGES[ref.triggerKind].path
	}
}

/** The page item a list page's row names, or undefined for a page that lists none. */
export function pageItemForListPath(pagePath: string, path: string): PageItemRef | undefined {
	const clean = stripBase(pagePath)
	if (clean === VARIABLES_PATH) return { kind: 'variable', path }
	if (clean === RESOURCES_PATH) return { kind: 'resource', path }
	if (clean === SCHEDULES_PATH) return { kind: 'schedule', path }
	const trigger = Object.entries(TRIGGER_PAGES).find(([, p]) => p.path === clean)
	return trigger ? { kind: 'trigger', triggerKind: trigger[0] as TriggerKind, path } : undefined
}

const PAGE_ITEM_ROUTE = /^pageitem:(variable|resource|schedule|trigger\.([a-z]+))\/([^?#]+)$/

// A scheme rather than a path, like artifacts: the tab mounts the item's editor in process,
// so there is no page a frame could load. The path is encoded whole, so its slashes cannot
// be read as part of the scheme.
export function pageItemUrl(ref: PageItemRef): string {
	const kind = ref.kind === 'trigger' ? `trigger.${ref.triggerKind}` : ref.kind
	return `pageitem:${kind}/${encodeURIComponent(ref.path)}`
}

export function parsePageItemRoute(url: string): PageItemRef | null {
	const m = url.match(PAGE_ITEM_ROUTE)
	if (!m) return null
	let path: string
	try {
		path = decodeURIComponent(m[3])
	} catch {
		return null
	}
	if (m[2] !== undefined) {
		if (!(m[2] in TRIGGER_PAGES)) return null
		return { kind: 'trigger', triggerKind: m[2] as TriggerKind, path }
	}
	return { kind: m[1] as 'variable' | 'resource' | 'schedule', path }
}

/** Singular human name of a page item's kind, e.g. "Kafka trigger". */
export function pageItemKindLabel(ref: PageItemRef): string {
	switch (ref.kind) {
		case 'variable':
			return 'Variable'
		case 'resource':
			return 'Resource'
		case 'schedule':
			return 'Schedule'
		case 'trigger':
			return TRIGGER_PAGES[ref.triggerKind].label.replace(/s$/, '')
	}
}

/** Label a trigger list page from its (base-stripped) pathname, or undefined. */
export function triggerLabelForPath(path: string): string | undefined {
	const clean = stripBase(path)
	return Object.values(TRIGGER_PAGES).find((t) => t.path === clean)?.label
}

export const pageKey = (path: string) => `page:${path}`
export const pageHref = (path: string) => `${base}${path}`

/** Strip the deployment base prefix (and any query/hash) from a preview path
 * so it can be matched against `PREVIEW_PAGES` / parsed as an item route. */
export function stripBase(path: string): string {
	let p = path.split('?')[0].split('#')[0]
	if (base && p.startsWith(base)) p = p.slice(base.length)
	return p || '/'
}

export type PreviewItemRoute = { kind: WorkspaceItemKind; raw_app: boolean; itemPath: string }

// Parse a preview URL/pathname into the workspace item it edits, or null for a
// non-item page (home, runs, …). Shared by the breadcrumb (drill segments) and
// `previewRouter`'s tab resolver so both agree on what counts as an item route.
export function parsePreviewItemRoute(fullPath: string): PreviewItemRoute | null {
	const p = stripBase(fullPath)
	const m = p.match(/^\/(scripts|flows|apps|apps_raw)\/(?:edit|get)\/(.+)$/)
	if (!m) return null
	const itemPath = decodeURIComponent(m[2])
	if (m[1] === 'scripts') return { kind: 'script', raw_app: false, itemPath }
	if (m[1] === 'flows') return { kind: 'flow', raw_app: false, itemPath }
	if (m[1] === 'apps_raw') return { kind: 'app', raw_app: true, itemPath }
	return { kind: 'app', raw_app: false, itemPath }
}
