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
import { isSessionPipelinesEnabled } from '$lib/components/copilot/chat/global/pipelineGate'
import type { SessionTargetKind } from './sessionRuntime.svelte'

/** What the preview breadcrumb picker can route to: a static workspace page
 * or a workspace item (script/flow/app). The sessions page turns either into
 * an iframe URL. */
export type PreviewTarget =
	| { type: 'page'; href: string; label: string }
	| { type: 'item'; item: WorkspaceItem }
	| { type: 'artifact'; id: string; name: string }

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

// Match a base-stripped preview pathname to a known page, for breadcrumb
// labelling + picker highlight. Exact match; '/' only matches home.
export function matchPreviewPage(path: string): PreviewPage | undefined {
	const clean = stripBase(path)
	return PREVIEW_PAGES.find((p) => p.path === clean)
}

/** Human label for a preview tab's location — the workspace page name, trigger
 * page, run detail, or item path. Shared by the sessions tab strip and the
 * close_page matcher so both name a tab the same way. */
export function previewLocationLabel(url: string): string {
	const artifact = parseArtifactRoute(url)
	if (artifact) return artifact.name || 'Artifact'
	const page = matchPreviewPage(url)
	if (page) return page.label
	const trigger = triggerLabelForPath(url)
	if (trigger) return trigger
	const run = stripBase(url).match(/^\/run\/([^/?#]+)/)
	if (run) return `Run ${decodeURIComponent(run[1]).slice(0, 8)}`
	const pipelineFolder = parsePipelineRoute(url)
	if (pipelineFolder) return pipelineFolder
	const parsed = parsePreviewItemRoute(url)
	if (parsed) return parsed.itemPath.split('/').pop() ?? parsed.itemPath
	return stripBase(url)
}

/** The friendly display leaf for a preview tab, or `undefined` to fall back to
 * `previewLocationLabel`. A never-deployed script / flow / raw app is parked at a
 * throwaway `…/draft_<uuid>` storage path while its editor shows a friendly name
 * (auto-generated or typed); pass that `friendlyPath` — the live cell's
 * `draft_path`/`path` — to label the tab by its leaf instead of the uuid. Returns
 * `undefined` for a deployed item (real storage path) or when the friendly path
 * is itself a placeholder. Display-only: the tab's URL keeps the storage path. */
export function draftFriendlyLeaf(
	storagePath: string,
	friendlyPath: string | undefined
): string | undefined {
	if (!storagePath.split('/').pop()?.startsWith('draft_')) return undefined
	const leaf = friendlyPath?.split('/').pop()
	return leaf && !leaf.startsWith('draft_') ? leaf : undefined
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

// A `/pipeline/<folder>` route is the data-pipeline graph editor for that folder
// (the folder is a single path segment, not a workspace item path). The bare
// `/pipeline` list page is not an editor. Returns the folder name, or null.
export function parsePipelineRoute(fullPath: string): string | null {
	const m = stripBase(fullPath).match(/^\/pipeline\/([^/?#]+)/)
	return m ? decodeURIComponent(m[1]) : null
}

// The id (before the hash) is the artifact's stable routing identity; the name rides in
// the hash so the tab strip labels it without a store lookup.
export function parseArtifactRoute(url: string): { id: string; name: string } | null {
	const m = url.match(/^artifact:([^#]+)(?:#(.*))?$/)
	if (!m) return null
	return { id: decodeURIComponent(m[1]), name: m[2] ? decodeURIComponent(m[2]) : '' }
}

export function artifactUrl(id: string, name: string): string {
	return `artifact:${encodeURIComponent(id)}#${encodeURIComponent(name)}`
}

/** Drill-picker leaf key for an artifact, shared by the picker tree and the
 * active-tab highlight so a pick and a highlight agree on identity. */
export const artifactKey = (id: string) => `artifact:${id}`

export const isArtifactKey = (key: string) => key.startsWith('artifact:')

// How a preview tab should render: as an in-process live editor or an iframe
// fallback. Any editable item of a wrappable kind (script, flow, raw app) mounts
// its per-(kind,path) cell editor; a `/pipeline/<folder>` route mounts the
// data-pipeline graph editor (single, shared runtime.pipelineEditorState — `path`
// is the folder); everything else (static pages, regular drag-and-drop apps, any
// other route) stays an iframe.
export type PreviewSlot =
	| { kind: 'editor'; editorKind: SessionTargetKind | 'pipeline'; path: string }
	| { kind: 'artifact'; id: string }
	| { kind: 'iframe' }

export function resolvePreviewTab(url: string): PreviewSlot {
	const artifact = parseArtifactRoute(url)
	if (artifact) return { kind: 'artifact', id: artifact.id }
	// While session pipelines are gated, a /pipeline/<folder> tab stays a plain
	// iframe: mounting the in-realm pipeline editor would register the pipeline
	// canvas tools on the session chat, un-gating it through the back door.
	const pipelineFolder = parsePipelineRoute(url)
	if (pipelineFolder && isSessionPipelinesEnabled()) {
		return { kind: 'editor', editorKind: 'pipeline', path: pipelineFolder }
	}
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
	return { kind: 'editor', editorKind, path: route.itemPath }
}
