import { base } from '$lib/base'
import {
	Home,
	Play,
	DollarSign,
	Boxes,
	Calendar,
	Database,
	FolderOpen,
	GitCompareArrows,
	Users,
	Settings,
	ScrollText
} from 'lucide-svelte'
import type { DrillIcon } from '$lib/components/drillPicker'
import { buildRunsFilterSearchbarSchema } from '$lib/components/runs/runsFilter'
import { buildSchedulesFilterSchema } from '$lib/components/schedules/schedulesFilter'
import { buildVariablesFilterSchema } from '$lib/components/variables/variablesFilter'
import { buildResourcesFilterSchema } from '$lib/components/resources/resourcesFilter'
import { buildAssetsFilterSchema } from '$lib/components/assets/assetsFilter'
import { COMPARE_ITEMS_PARAM } from './modifiedItemsMask'
import { normalizePipelineFolder } from '$lib/utils/pipelineFolder'
import type { WorkspaceItem, WorkspaceItemKind } from '$lib/components/workspacePicker'
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

// The Compare & Deploy review page. Kept out of PREVIEW_PAGES (it's not a picker
// destination — it's reached through the chat's open_page tool or a session's
// Review button) but known here so preview tabs label it and reuse it on
// param changes like the curated pages.
export const COMPARE_PAGE: PreviewPage = {
	label: 'Compare & Deploy',
	path: '/forks/compare',
	icon: GitCompareArrows
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

// Workspace list pages that deep-link one row through the hash. Resources route
// theirs through an extra `/resource/` segment. Nothing else may be read that way:
// a legacy drag-and-drop app hands its hash to the app itself as `context.hash`,
// so treating that as a row would describe app state as a workspace item.
const DRAWER_ANCHOR_PAGES = ['/schedules', '/variables', '/resources'] as const

/** The workspace item whose drawer a preview location has open, or undefined when
 * the page doesn't deep-link rows (or none is anchored). Takes the location with
 * its suffix — `stripBaseKeepingSuffix` output or a raw href. */
export function drawerAnchorFor(location: string): string | undefined {
	const hashAt = location.indexOf('#')
	if (hashAt < 0) return undefined
	const route = stripBase(location)
	const known =
		(DRAWER_ANCHOR_PAGES as readonly string[]).includes(route) ||
		Object.values(TRIGGER_PAGES).some((p) => p.path === route)
	if (!known) return undefined
	return location.slice(hashAt + 1).replace(/^\/resource\//, '') || undefined
}

// Query params the preview host injects into an iframe URL (`nomenubar` hides the nav,
// `workspace` scopes the page). Never part of what a location means.
const INJECTED_PARAMS = ['nomenubar', 'workspace'] as const

/** Drop the params the preview host injects, so a location observed in the frame can be
 * compared with the one that was commanded (which never carries them). */
export function canonicalizeObservedLoc(loc: string): string {
	// An artifact is a scheme, not a path — `new URL` would happily parse it and hand
	// back a pathname with the scheme gone.
	if (parseArtifactRoute(loc)) return loc
	try {
		const u = new URL(loc, 'http://_')
		for (const p of INJECTED_PARAMS) u.searchParams.delete(p)
		return u.pathname + u.search + u.hash
	} catch {
		return loc
	}
}

// The query params belonging to the request; every other one the page wrote into its own
// URL (`filter_path_of`, `page`/`perPage`) and must not read as a view. Where a page
// declares a filter schema — what it hands FilterSearchbar — that schema is its own list
// of the names, with every option on so the set is its whole vocabulary rather than one
// viewer's subset; the rest read their params straight off the URL and are written out.
// Built on first use: only the key names are ever needed, and this module is imported by
// the Runs page and by every trigger drawer.
let requestParams: Record<string, readonly string[]> | undefined

function pageRequestParamTable(): Record<string, readonly string[]> {
	return (requestParams ??= {
		'/runs': Object.keys(
			buildRunsFilterSearchbarSchema({
				paths: [],
				usernames: [],
				folders: [],
				jobTriggerKinds: [],
				isSuperAdminOrDevops: true,
				isAdminsWorkspace: true
			})
		),
		'/schedules': Object.keys(
			buildSchedulesFilterSchema({ paths: [], scriptPaths: [], showUserFoldersFilter: true })
		),
		'/variables': Object.keys(
			buildVariablesFilterSchema({ paths: [], owners: [], showUserFoldersFilter: true })
		),
		'/resources': Object.keys(
			buildResourcesFilterSchema({
				paths: [],
				resourceTypes: [],
				owners: [],
				showUserFoldersFilter: true
			})
		),
		'/assets': Object.keys(buildAssetsFilterSchema({ paths: [], assetKinds: [] })),
		'/audit_logs': ['username', 'operation', 'resource'],
		'/workspace_settings': ['tab'],
		[COMPARE_PAGE.path]: ['workspace_id', 'mode', COMPARE_ITEMS_PARAM]
	})
}

/** The query params a request can set on `path` — empty for a page that takes none. */
export function pageRequestParams(path: string): readonly string[] {
	return pageRequestParamTable()[stripBase(path)] ?? []
}

/** What a preview location means, read against the page it points at. */
export type PreviewLocation = {
	/** What two locations must share to be the same tab. */
	identity: string
	/** The view within that page: only the params a request could have set. */
	view: string
	/** The row the page deep-links, `''` where the hash is the page's own state. */
	anchor: string
}

/** Decompose a preview location into what it means. Everything comparing two locations
 * goes through this: a query or a hash means something different per class of page, and
 * answering that at the call site is how a tab ends up duplicated, reloaded, or reported
 * as showing a row it is not. The page classes live here and only here. */
export function describeLocation(loc: string): PreviewLocation {
	const artifact = parseArtifactRoute(loc)
	if (artifact) return { identity: `artifact:${artifact.id}`, view: '', anchor: '' }
	const canonical = canonicalizeObservedLoc(loc)
	const path = stripBase(canonical)
	const bare = canonical.split('#')[0]
	const query = bare.includes('?') ? bare.slice(bare.indexOf('?') + 1) : ''
	return {
		identity: path,
		view: requestedParams(query, pageRequestParamTable()[path]),
		anchor: drawerAnchorFor(canonical) ?? ''
	}
}

// By content, never by the raw string: a page hands its params back in its own order and
// re-encodes what it was given (`path=f/a` arrives as `path=f%2Fa`), and neither is a
// change of view. Re-encoded on the way out so a value holding a delimiter stays one
// pair — decoded, `?arg=x%26result%3Dy` and `?arg=x&result=y` read alike.
function requestedParams(query: string, allowed: readonly string[] | undefined): string {
	if (!allowed?.length) return ''
	const parts: string[] = []
	new URLSearchParams(query).forEach((v, k) => {
		if (allowed.includes(k)) parts.push(`${encodeURIComponent(k)}=${encodeURIComponent(v)}`)
	})
	return parts.sort().join('&')
}

/** Whether two preview locations show the same thing: same page, view and row. */
export function sameView(a: string, b: string): boolean {
	const x = describeLocation(a)
	const y = describeLocation(b)
	return x.identity === y.identity && x.view === y.view && x.anchor === y.anchor
}

// Filters whose value addresses a workspace object — a path, an owner, a kind, a state,
// a timestamp — and so may be repeated to the model. A filter absent here keeps its name
// and loses its value, because the rest search *over* content: the free-text box, a job's
// arguments or result, a variable's or resource's value. Withholding by default means a
// filter added later leaks nothing until it is listed deliberately.
const ADDRESSING_PARAMS = new Set([
	'path',
	'path_start',
	'schedule_path',
	'script_path',
	'asset_path',
	'usage_path',
	'owner',
	'user',
	'username',
	'folder',
	'worker',
	'tag',
	'label',
	'resource_type',
	'asset_kinds',
	'job_kinds',
	'job_trigger_kind',
	'operation',
	'resource',
	'concurrency_key',
	'status',
	'min_ts',
	'max_ts',
	'timeframe',
	'all_workspaces',
	'show_skipped',
	'show_future_jobs',
	'resolved',
	'user_folders_only',
	'columns',
	'tab',
	'workspace_id',
	'mode',
	COMPARE_ITEMS_PARAM
])

/** How a preview location may be described to the model. Reassembled from the parts this
 * module recognizes, never passed through whole: an iframe tab can host a legacy app,
 * whose hash is app state its author chose, and a page can carry both filters that
 * address an object and filters that search its contents. The chat has no redaction
 * boundary of its own, so only the addressing ones keep their value. */
export function previewLocationContext(loc: string): {
	label: string
	location: string
	open?: string
} {
	const { identity, anchor } = describeLocation(loc)
	const bare = canonicalizeObservedLoc(loc).split('#')[0]
	const query = bare.includes('?') ? bare.slice(bare.indexOf('?') + 1) : ''
	const declared = pageRequestParams(identity)
	const filters: string[] = []
	new URLSearchParams(query).forEach((v, k) => {
		if (declared.includes(k)) filters.push(ADDRESSING_PARAMS.has(k) ? `${k}=${v}` : k)
	})
	return {
		// Labels come from route shape (page name, trigger kind, run id, item leaf), so
		// they carry no query or hash of their own.
		label: previewLocationLabel(loc),
		location: identity + (filters.length ? `?${filters.sort().join('&')}` : ''),
		open: anchor || undefined
	}
}

/** Like `stripBase`, but keeps the query and hash: a list page's `?filters` and
 * its `#<path>` (the row whose drawer is open) are what the location says beyond
 * the page's name. Not for route matching — use `stripBase` for that. */
export function stripBaseKeepingSuffix(path: string): string {
	const bare = path.split('?')[0].split('#')[0]
	return stripBase(bare) + path.slice(bare.length)
}

// Match a base-stripped preview pathname to a known page, for breadcrumb
// labelling + picker highlight. Exact match; '/' only matches home.
export function matchPreviewPage(path: string): PreviewPage | undefined {
	const clean = stripBase(path)
	return PREVIEW_PAGES.find((p) => p.path === clean)
}

/** Match a preview href to a page whose tab should be re-pointed in place when
 * only its query params change (the open_page filter-change behavior): the
 * curated pages plus the compare page. Trigger pages are deliberately not
 * matched — they take the generic path in `SessionPreviewTabs.open`, which
 * dedupes on the location ignoring the hash and re-points the tab it finds. */
export function matchReusablePage(href: string): PreviewPage | undefined {
	if (stripBase(href) === COMPARE_PAGE.path) return COMPARE_PAGE
	return matchPreviewPage(href)
}

/** Human label for a preview tab's location — the workspace page name, trigger
 * page, run detail, or item path. Shared by the sessions tab strip and the
 * close_page matcher so both name a tab the same way. */
export function previewLocationLabel(url: string): string {
	const artifact = parseArtifactRoute(url)
	if (artifact) return artifact.name || 'Artifact'
	const page = matchReusablePage(url)
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

/** The display name for an item, from what a lister or a live editor cell knows
 * about it: its summary when set, else the typed/auto name of an item parked at a
 * `…/draft_<uuid>` storage path. `undefined` when neither applies, leaving the
 * caller on `previewLocationLabel`. Shared by the live editor's tab stamp and the
 * sessions page's pre-mount lookup so one tab can't be named two ways depending
 * on which of them got there first. */
export function itemDisplayName(
	storagePath: string,
	friendlyPath: string | undefined,
	summary: string | undefined
): string | undefined {
	return summary?.trim() || draftFriendlyLeaf(storagePath, friendlyPath)
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

// The place inside a previewed flow editor its tab URL asks for (`?selected=`,
// the same param the full-page flow editor reads). Live editors are mounted in
// process rather than in an iframe, so the host has to read this off the tab URL
// and seed the editor with it.
export function parsePreviewSelectedId(url: string): string | undefined {
	try {
		return new URL(url, 'http://_').searchParams.get('selected') || undefined
	} catch {
		return undefined
	}
}

// A `/pipeline/<folder>` route is the data-pipeline graph editor for that folder
// (the folder is a single path segment, not a workspace item path). The bare
// `/pipeline` list page is not an editor. Returns the folder name, or null.
export function parsePipelineRoute(fullPath: string): string | null {
	const m = stripBase(fullPath).match(/^\/pipeline\/([^/?#]+)/)
	return m ? normalizePipelineFolder(decodeURIComponent(m[1])) : null
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
	const pipelineFolder = parsePipelineRoute(url)
	if (pipelineFolder) {
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
