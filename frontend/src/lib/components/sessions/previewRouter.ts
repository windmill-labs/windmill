import {
	ASSETS_PATH,
	AUDIT_LOGS_PATH,
	FOLDERS_PATH,
	GROUPS_PATH,
	pageKey,
	pageHref,
	parsePreviewItemRoute,
	RESOURCES_PATH,
	RUNS_PATH,
	SCHEDULES_PATH,
	stripBase,
	VARIABLES_PATH,
	WORKSPACE_SETTINGS_PATH,
	triggerLabelForPath,
	TRIGGER_PAGES,
	type PreviewItemRoute,
	type TriggerKind
} from './previewPaths'
// Re-exported so the preview code that already reads locations through this module keeps
// one import, while a caller needing only a path can reach for the leaf instead.
export {
	pageKey,
	pageHref,
	parsePreviewItemRoute,
	stripBase,
	TRIGGER_PAGES,
	type PreviewItemRoute,
	type TriggerKind
}
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
import type { WorkspaceItem } from '$lib/components/workspacePicker'
import type { SessionTargetKind } from './sessionRuntime.svelte'

/**
 * Which version of an artifact an opener wants on screen: a number pins that one, `'latest'`
 * drops any pin, and omitting it leaves the reader where they are. `undefined` cannot double
 * as `'latest'` — every artifact tool re-opens the document it just wrote, so treating that
 * as a request to move would yank a reader out of the version they chose on each edit.
 */
export type ArtifactVersionTarget = number | 'latest'

/** What the preview breadcrumb picker can route to: a static workspace page
 * or a workspace item (script/flow/app). The sessions page turns either into
 * an iframe URL. */
export type PreviewTarget =
	| { type: 'page'; href: string; label: string }
	| { type: 'item'; item: WorkspaceItem }
	| { type: 'artifact'; id: string; name: string; version?: ArtifactVersionTarget }

export type PreviewPage = { label: string; path: string; icon: DrillIcon }

// Core workspace-level destinations the preview can route to. Intentionally
// curated — the main pages, not every trigger sub-page (those live behind
// EE/feature gating in SidebarContent and aren't worth duplicating here).
export const PREVIEW_PAGES: PreviewPage[] = [
	{ label: 'Home', path: '/', icon: Home },
	{ label: 'Runs', path: RUNS_PATH, icon: Play },
	{ label: 'Variables', path: VARIABLES_PATH, icon: DollarSign },
	{ label: 'Resources', path: RESOURCES_PATH, icon: Boxes },
	{ label: 'Schedules', path: SCHEDULES_PATH, icon: Calendar },
	{ label: 'Assets', path: ASSETS_PATH, icon: Database },
	{ label: 'Folders', path: FOLDERS_PATH, icon: FolderOpen },
	{ label: 'Groups', path: GROUPS_PATH, icon: Users },
	{ label: 'Workspace settings', path: WORKSPACE_SETTINGS_PATH, icon: Settings },
	{ label: 'Audit logs', path: AUDIT_LOGS_PATH, icon: ScrollText }
]

// The Compare & Deploy review page. Kept out of PREVIEW_PAGES (it's not a picker
// destination — it's reached through the chat's open_page tool or a session's
// Review button) but known here so preview tabs label it and reuse it on
// param changes like the curated pages.
export const COMPARE_PAGE: PreviewPage = {
	label: 'Compare & Deploy',
	path: '/forks/compare',
	icon: GitCompareArrows
}

// Workspace list pages that deep-link one row through the hash. Resources route
// theirs through an extra `/resource/` segment. Nothing else may be read that way:
// a legacy drag-and-drop app hands its hash to the app itself as `context.hash`,
// so treating that as a row would describe app state as a workspace item.
const DRAWER_ANCHOR_PAGES = [SCHEDULES_PATH, VARIABLES_PATH, RESOURCES_PATH] as const

/** The workspace item whose drawer a preview location has open, or undefined when
 * the page doesn't deep-link rows (or none is anchored). Takes the location with
 * its suffix — a raw href, or an observed location. */
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
// URL (`filter_path_of`, `page`/`perPage`) and must not read as a view. A page's own
// filter schema is the list, with every option on so the set is its whole vocabulary and
// not one viewer's subset. Built on first use — every trigger drawer imports this module.
let requestParams: Record<string, readonly string[]> | undefined

function pageRequestParamTable(): Record<string, readonly string[]> {
	return (requestParams ??= {
		[RUNS_PATH]: Object.keys(
			buildRunsFilterSearchbarSchema({
				paths: [],
				usernames: [],
				folders: [],
				jobTriggerKinds: [],
				isSuperAdminOrDevops: true,
				isAdminsWorkspace: true
			})
		),
		[SCHEDULES_PATH]: Object.keys(
			buildSchedulesFilterSchema({ paths: [], scriptPaths: [], showUserFoldersFilter: true })
		),
		[VARIABLES_PATH]: Object.keys(
			buildVariablesFilterSchema({ paths: [], owners: [], showUserFoldersFilter: true })
		),
		[RESOURCES_PATH]: Object.keys(
			buildResourcesFilterSchema({
				paths: [],
				resourceTypes: [],
				owners: [],
				showUserFoldersFilter: true
			})
		),
		[ASSETS_PATH]: Object.keys(buildAssetsFilterSchema({ paths: [], assetKinds: [] })),
		[AUDIT_LOGS_PATH]: ['username', 'operation', 'resource'],
		[WORKSPACE_SETTINGS_PATH]: ['tab'],
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

// Params a page restores from the user's stored preference whenever the URL it loads with
// says nothing about them. Requestable like any other filter, so they stay in the view —
// but a frame carrying one the request never mentioned is the page's own doing, and
// loading over it lands on a page that seeds it straight back, scroll position gone.
const PAGE_SEEDED_PARAMS: Record<string, readonly string[]> = {
	[RUNS_PATH]: ['job_trigger_kind', 'show_future_jobs']
}

/** Whether the frame at `observed` is already showing what `commanded` asks for: same
 * page, same row, and every filter the request names carrying the value it named. */
export function showsView(observed: string, commanded: string): boolean {
	const x = describeLocation(observed)
	const y = describeLocation(commanded)
	if (x.identity !== y.identity || x.anchor !== y.anchor) return false
	if (x.view === y.view) return true
	const seeded = PAGE_SEEDED_PARAMS[x.identity]
	if (!seeded?.length) return false
	// Both views come out of `requestedParams` already sorted and encoded, so they compare
	// as strings once the seeded params are dropped. One direction only: the page may add
	// what the request left out, never drop what it asked for — dropping in both would let
	// a tab answer a request it does not satisfy.
	const pairs = (view: string) => (view ? view.split('&') : [])
	const keyOf = (pair: string) => decodeURIComponent(pair.split('=')[0])
	const asked = new Set(pairs(y.view).map(keyOf))
	return (
		pairs(x.view)
			.filter((pair) => !seeded.includes(keyOf(pair)) || asked.has(keyOf(pair)))
			.join('&') === y.view
	)
}

// Filters whose value addresses a workspace object — a path, owner, kind, state, time —
// and so may be repeated to the model. Any other keeps its name and loses its value: the
// rest search *over* content (the free-text box, a job's result, a resource's value), and
// withholding by default means a filter added later leaks nothing until it is listed.
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

const CONTEXT_FIELD_MAX = 300

/** Collapse a value to one field of prompt text or a tool result. Both are line-oriented
 * formats with no escaping, so a value carrying a newline writes a line of its own — and
 * these values are decoded out of URLs, which are attacker-shaped input the moment a link
 * can be shared. Length is capped too: a path this long tells the model nothing. */
export function promptSafe(text: string): string {
	// C0 and DEL, plus the Unicode terminators a renderer also breaks a line on — these
	// values are percent-decoded, so `%E2%80%A8` arrives as a real U+2028.
	// eslint-disable-next-line no-control-regex
	return text
		.replace(/[\u0000-\u001f\u007f\u0085\u2028\u2029]+/g, ' ')
		.trim()
		.slice(0, CONTEXT_FIELD_MAX)
}

/** How a preview location may be described to the model: reassembled from the parts this
 * module recognizes, never passed through whole. A tab can host a legacy app whose hash is
 * app state, and a filter can search contents rather than address them — so only the
 * addressing ones keep their value, the chat having no redaction boundary of its own. */
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
		// Re-encoded for the same reason the view is: decoded, a value holding `&` or `=`
		// reads as another filter entirely.
		if (declared.includes(k)) {
			const key = encodeURIComponent(k)
			filters.push(ADDRESSING_PARAMS.has(k) ? `${key}=${encodeURIComponent(v)}` : key)
		}
	})
	return {
		// Labels come from route shape (page name, trigger kind, run id, item leaf), so
		// they carry no query or hash of their own — but a run id and an item leaf are
		// decoded out of the path, so they still reach here as free text.
		label: promptSafe(previewLocationLabel(loc)),
		location: promptSafe(identity + (filters.length ? `?${filters.sort().join('&')}` : '')),
		open: anchor ? promptSafe(anchor) : undefined
	}
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

// The id (before the query) is the artifact's stable routing identity; the name rides in the
// hash so the tab strip labels it without a store lookup, and the version in the query so the
// tab persists it.
export function parseArtifactRoute(
	url: string
): { id: string; name: string; version?: number } | null {
	const m = url.match(/^artifact:([^?#]+)(?:\?v=(\d+))?(?:#(.*))?$/)
	if (!m) return null
	// Mirror artifactUrl: a version it would not stamp does not read back as a pin.
	const version = Number(m[2])
	return {
		id: decodeURIComponent(m[1]),
		name: m[3] ? decodeURIComponent(m[3]) : '',
		version: version > 0 ? version : undefined
	}
}

export function artifactUrl(id: string, name: string, version?: number): string {
	// Only stamp a version parseArtifactRoute can read back: this url is persisted with the
	// tab, so one that round-trips to null would come back as an unopenable tab every reload.
	// Safe integers specifically — from 1e21 up a number interpolates as `1e+21`, which the
	// digits-only parser rejects outright.
	const pinned =
		version !== undefined && Number.isSafeInteger(version) && version > 0 ? `?v=${version}` : ''
	return `artifact:${encodeURIComponent(id)}${pinned}#${encodeURIComponent(name)}`
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
	| { kind: 'artifact'; id: string; version?: number }
	| { kind: 'iframe' }

export function resolvePreviewTab(url: string): PreviewSlot {
	const artifact = parseArtifactRoute(url)
	if (artifact) return { kind: 'artifact', id: artifact.id, version: artifact.version }
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
