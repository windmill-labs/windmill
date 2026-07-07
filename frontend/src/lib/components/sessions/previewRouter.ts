import { base } from '$lib/base'
import {
	Home,
	Play,
	DollarSign,
	Boxes,
	Calendar,
	Database,
	FolderOpen,
	Users,
	Settings,
	ScrollText
} from 'lucide-svelte'
import type { DrillIcon } from '$lib/components/drillPicker'
import type { WorkspaceItem, WorkspaceItemKind } from '$lib/components/workspacePicker'
import type { SessionTarget } from './sessionState.svelte'
import type { SessionTargetKind } from './sessionRuntime.svelte'

/** What the preview breadcrumb picker can route to: a static workspace page
 * or a workspace item (script/flow/app). The sessions page turns either into
 * an iframe URL. */
export type PreviewTarget =
	| { type: 'page'; href: string; label: string }
	| { type: 'item'; item: WorkspaceItem }

export type PreviewPage = { label: string; path: string; icon: DrillIcon }

// Core workspace-level destinations the preview can route to. Intentionally
// curated — the main pages, not every trigger sub-page (those live behind
// EE/feature gating in SidebarContent and aren't worth duplicating here).
export const PREVIEW_PAGES: PreviewPage[] = [
	{ label: 'Home', path: '/', icon: Home },
	{ label: 'Runs', path: '/runs', icon: Play },
	{ label: 'Variables', path: '/variables', icon: DollarSign },
	{ label: 'Resources', path: '/resources', icon: Boxes },
	{ label: 'Schedules', path: '/schedules', icon: Calendar },
	{ label: 'Assets', path: '/assets', icon: Database },
	{ label: 'Folders', path: '/folders', icon: FolderOpen },
	{ label: 'Groups', path: '/groups', icon: Users },
	{ label: 'Workspace settings', path: '/workspace_settings', icon: Settings },
	{ label: 'Audit logs', path: '/audit_logs', icon: ScrollText }
]

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

// Match a base-stripped preview pathname to a known page, for breadcrumb
// labelling + picker highlight. Exact match; '/' only matches home.
export function matchPreviewPage(path: string): PreviewPage | undefined {
	const clean = stripBase(path)
	return PREVIEW_PAGES.find((p) => p.path === clean)
}

export type PreviewItemRoute = { kind: WorkspaceItemKind; raw_app: boolean; itemPath: string }

// Parse a preview URL/pathname into the workspace item it edits, or null for a
// non-item page (home, runs, …). Shared by the breadcrumb (drill segments) and
// the tab resolver below so both agree on what counts as an item route.
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

// How a preview tab should render: as an in-process live editor (sharing the
// session runtime's store) or as an iframe fallback. Only the three kinds with
// existing editor wrappers — and only the tab matching the session's target —
// resolve to 'editor'; everything else (static pages, regular drag-and-drop
// apps, any other item) stays an iframe.
export type PreviewSlot =
	| { kind: 'editor'; editorKind: SessionTargetKind; path: string }
	| { kind: 'iframe' }

export function resolvePreviewTab(url: string, target: SessionTarget | undefined): PreviewSlot {
	const route = parsePreviewItemRoute(url)
	if (!route) return { kind: 'iframe' }
	const editorKind: SessionTargetKind | undefined =
		route.kind === 'script'
			? 'script'
			: route.kind === 'flow'
				? 'flow'
				: route.kind === 'app' && route.raw_app
					? 'raw_app'
					: undefined
	if (!editorKind) return { kind: 'iframe' }
	// SessionRuntime holds one load slot per kind, so only the tab pointing at the
	// session's own target claims it as a live editor; any other item previews as
	// an iframe (the "one live editor per session" rule).
	if (!target || target.kind !== editorKind || target.path !== route.itemPath) {
		return { kind: 'iframe' }
	}
	return { kind: 'editor', editorKind, path: route.itemPath }
}
