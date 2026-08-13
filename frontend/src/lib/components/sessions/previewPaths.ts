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
