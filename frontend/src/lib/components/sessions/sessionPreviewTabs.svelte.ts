import { base } from '$lib/base'
import { randomUUID } from '$lib/utils/uuid'
import { editPathFor, type WorkspaceItem } from '$lib/components/workspacePicker'
import {
	matchPreviewPage,
	parsePipelineRoute,
	parsePreviewItemRoute,
	previewLocationLabel,
	resolvePreviewTab,
	stripBase,
	type PreviewTarget
} from './previewRouter'
import type { SessionPreviewTab, SessionTarget } from './sessionState.svelte'

// The single live owner of a session's preview tabs. Runs behind a small
// interface both the sessions page (renderer) and the `open_preview` tool cross,
// so there is exactly one live copy of the tab model instead of three drifting
// ones synced by effects. Persistence (and cell pruning) are injected as an
// adapter, so the class is pure runes with no sessionState / IDB coupling
// (mirrors PipelineEditorState). Held on SessionRuntime.previewTabs.

export type PreviewTabsSnapshot = {
	tabs: SessionPreviewTab[]
	activeId: string
	collapsed: boolean
	previewSize?: number
}

export type PreviewTabsAdapter = {
	// Write-behind the full tab model onto the durable backing (debounced by the
	// owner). Fire-and-forget.
	persist: (snapshot: PreviewTabsSnapshot) => void
	// Fired synchronously on every tab-set change, so the runtime can drop editor
	// cells no open tab references anymore (a closed / navigated-away item).
	onTabsChanged?: () => void
}

// True when a tab's URL is the live editor for a specific editable item. Every
// editable route resolves to an editor, so this doubles as the "same item" dedupe
// test in open()/navigate().
function isEditorTabFor(url: string, target: SessionTarget): boolean {
	const slot = resolvePreviewTab(url)
	return slot.kind === 'editor' && slot.editorKind === target.kind && slot.path === target.path
}

// URL a tab should load for a destination: a page's href, or an item's edit route.
function targetUrl(target: PreviewTarget): string {
	return target.type === 'page' ? target.href : `${base}${editPathFor(target.item)}`
}

// Point a tab at a new destination. Clears `friendlyLabel` (bound to the previous
// editor's item): a new editor re-stamps it, and navigating to a plain page must
// drop the stale name so the tab falls back to the location label.
function retargetTab(tab: SessionPreviewTab, url: string): void {
	tab.url = url
	tab.loc = url
	tab.friendlyLabel = undefined
}

// Strip the query params the sessions preview injects into iframe URLs
// (`nomenubar` to hide the nav, `workspace` to scope the page): they aren't part
// of the canonical page URL. The observed `loc` must drop them to stay symmetric
// with `url` (targetUrl, which never carries them), else reopening the same page
// spawns a duplicate tab instead of focusing the existing one.
export function canonicalizeObservedLoc(loc: string): string {
	try {
		const u = new URL(loc, 'http://_')
		u.searchParams.delete('nomenubar')
		u.searchParams.delete('workspace')
		return u.pathname + u.search + u.hash
	} catch {
		return loc
	}
}

// The editor target a destination maps to, or undefined when it isn't an item we
// host live (static pages, legacy drag-and-drop apps). Drives the open()/navigate()
// dedupe — one editor tab per (kind, path).
function editorTargetFor(target: PreviewTarget): SessionTarget | undefined {
	if (target.type !== 'item') return undefined
	const item = target.item
	if (item.kind === 'script') return { kind: 'script', path: item.path }
	if (item.kind === 'flow') return { kind: 'flow', path: item.path }
	if (item.kind === 'app' && item.raw_app) return { kind: 'raw_app', path: item.path }
	return undefined
}

// Adapt a session editor target (`open_preview` tool arg) to a preview
// destination. A pipeline target's `path` is a folder name, not a workspace
// item — it maps to the `/pipeline/<folder>` route, which resolvePreviewTab
// mounts as the in-process graph editor.
export function previewTargetForSessionTarget(
	kind: SessionTarget['kind'],
	path: string
): PreviewTarget | undefined {
	if (kind === 'pipeline') {
		return { type: 'page', href: `${base}/pipeline/${encodeURIComponent(path)}`, label: path }
	}
	const item: WorkspaceItem =
		kind === 'raw_app'
			? { kind: 'app', raw_app: true, path, summary: '' }
			: { kind, path, summary: '' }
	return { type: 'item', item }
}

// Build the initial tab model for a session: its saved tabs, else empty. Default
// collapse: collapsed only for a session with nothing to preview.
export function hydratePreviewTabs(session: {
	previewTabs?: SessionPreviewTab[]
	activePreviewTabId?: string
	previewCollapsed?: boolean
	previewSize?: number
}): PreviewTabsSnapshot {
	// Saved tabs come straight from IndexedDB — drop malformed records (missing
	// id/url) and duplicate ids, which would break the page's keyed {#each}.
	const seen = new Set<string>()
	const tabs: SessionPreviewTab[] = []
	for (const t of session.previewTabs ?? []) {
		if (!t?.id || !t?.url || seen.has(t.id)) continue
		seen.add(t.id)
		// Rebuilt field-by-field so stray properties on old saved records (e.g. the
		// retired `pinned` flag) don't survive hydration and get persisted back.
		tabs.push({ id: t.id, url: t.url, loc: t.loc ?? t.url })
	}
	if (tabs.length > 0) {
		const wantActive = session.activePreviewTabId
		const activeId = wantActive && tabs.some((t) => t.id === wantActive) ? wantActive : tabs[0].id
		return {
			tabs,
			activeId,
			collapsed: session.previewCollapsed ?? false,
			previewSize: session.previewSize
		}
	}
	return {
		tabs: [],
		activeId: '',
		collapsed: session.previewCollapsed ?? true,
		previewSize: session.previewSize
	}
}

const FLUSH_DELAY_MS = 400

export class SessionPreviewTabs {
	// Each tab tracks two URLs: `url` is what we command the iframe to load
	// (changes only on an explicit open/navigate), `loc` the last observed
	// location. Keeping them separate lets a tab stay mounted — in-iframe
	// navigation updates `loc` only, so `url` (bound to `src`) never reloads.
	#tabs = $state<SessionPreviewTab[]>([])
	#activeId = $state('')
	#collapsed = $state(false)
	#previewSize = $state<number | undefined>(undefined)
	readonly #adapter: PreviewTabsAdapter
	readonly #flushDelay: number
	#flushHandle: ReturnType<typeof setTimeout> | undefined

	constructor(
		initial: PreviewTabsSnapshot,
		adapter: PreviewTabsAdapter,
		flushDelay = FLUSH_DELAY_MS
	) {
		this.#tabs = initial.tabs.map((t) => ({ ...t }))
		this.#activeId = initial.activeId
		this.#collapsed = initial.collapsed
		this.#previewSize = initial.previewSize
		this.#adapter = adapter
		this.#flushDelay = flushDelay
	}

	get tabs(): SessionPreviewTab[] {
		return this.#tabs
	}
	get activeId(): string {
		return this.#activeId
	}
	get activeTab(): SessionPreviewTab | undefined {
		return this.#tabs.find((t) => t.id === this.#activeId) ?? this.#tabs[0]
	}
	get collapsed(): boolean {
		return this.#collapsed
	}
	get previewSize(): number | undefined {
		return this.#previewSize
	}

	setPreviewSize(size: number): void {
		if (this.#previewSize === size) return
		this.#previewSize = size
		// A size change never touches the tab set, so skip the editor-cell prune
		// (onTabsChanged) and only schedule the debounced persist.
		this.#schedulePersist()
	}

	// Open — or focus, if already shown — a tab for a destination, and reveal the
	// panel. An editable item dedupes against the tab already hosting that same
	// (kind, path); anything else dedupes on the tab's observed location.
	open(target: PreviewTarget): { status: 'opened' | 'focused' } {
		const editorTarget = editorTargetFor(target)
		// A fresh session starts collapsed, so without this the tab opens behind a
		// collapsed panel and the user sees nothing change.
		this.#collapsed = false
		if (editorTarget) {
			// One editor tab per item: focus the tab already hosting this exact item.
			const existing = this.#tabs.find((t) => isEditorTabFor(t.url, editorTarget))
			if (existing) {
				this.#activeId = existing.id
				this.#flush()
				return { status: 'focused' }
			}
		}
		const url = targetUrl(target)
		// Pipeline previews all share one runtime.pipelineEditorState, so keep at
		// most one pipeline tab: re-point the existing one to the requested folder
		// rather than opening a second pipeline editor that would fight over the
		// shared state (`focused` when it already showed this folder, else `opened`
		// since the view now shows a different pipeline).
		const pipelineFolder = parsePipelineRoute(url)
		if (pipelineFolder) {
			const existing = this.#tabs.find((t) => parsePipelineRoute(t.url) !== null)
			if (existing) {
				const same = existing.url === url
				retargetTab(existing, url)
				this.#activeId = existing.id
				this.#flush()
				return { status: same ? 'focused' : 'opened' }
			}
		}
		// Focus the tab currently *showing* this destination instead of opening a
		// duplicate. Matched on the observed `loc`, not `url`: a tab that was
		// opened here but navigated away no longer counts as showing it.
		const shown = this.#tabs.find((t) => t.loc === url)
		if (shown) {
			this.#activeId = shown.id
			this.#flush()
			return { status: 'focused' }
		}
		const tab: SessionPreviewTab = { id: randomUUID(), url, loc: url }
		this.#tabs.push(tab)
		this.#activeId = tab.id
		this.#flush()
		return { status: 'opened' }
	}

	// Re-point the active tab at a destination (breadcrumb pick / in-editor link /
	// iframe-posted editor navigation).
	navigate(target: PreviewTarget): void {
		const t = this.#tabs.find((x) => x.id === this.#activeId)
		if (!t) return
		const editorTarget = editorTargetFor(target)
		if (editorTarget) {
			// Same dedupe as open(): if another tab already hosts this exact item,
			// focus it instead of re-pointing this one — two tabs for one item would
			// mount two editors racing the same (kind, path) cell.
			const existing = this.#tabs.find((x) => isEditorTabFor(x.url, editorTarget))
			if (existing && existing.id !== t.id) {
				this.#activeId = existing.id
				this.#flush()
				return
			}
		}
		const url = targetUrl(target)
		// Keep at most one pipeline tab (all share runtime.pipelineEditorState): if a
		// *different* tab already hosts a pipeline, retarget and focus it rather than
		// turning the active tab into a second pipeline editor racing the shared
		// state. Same invariant as open(); a no-op when the active tab is that tab.
		const pipelineFolder = parsePipelineRoute(url)
		if (pipelineFolder) {
			const existing = this.#tabs.find((x) => parsePipelineRoute(x.url) !== null)
			if (existing && existing.id !== t.id) {
				retargetTab(existing, url)
				this.#activeId = existing.id
				this.#flush()
				return
			}
		}
		retargetTab(t, url)
		this.#flush()
	}

	// Replace the whole tab model and reveal the panel. For re-pointing an
	// existing draft session at a new destination, where the current tabs
	// (persisted and/or live) still show the previous one.
	reset(tabs: SessionPreviewTab[], activeId: string): void {
		this.#tabs = tabs.map((t) => ({ ...t }))
		this.#activeId = activeId
		this.#collapsed = false
		this.#flush()
	}

	select(id: string): void {
		if (this.#activeId === id) return
		this.#activeId = id
		this.#flush()
	}

	// Reorder the tabs to the given id order (drag-and-drop). Ids absent from the
	// current set are ignored; any current tab the caller omitted is kept at the
	// end so a stale/partial order can never drop a tab. No-op if unchanged.
	reorder(orderedIds: string[]): void {
		const byId = new Map(this.#tabs.map((t) => [t.id, t]))
		const next: SessionPreviewTab[] = []
		for (const id of orderedIds) {
			const t = byId.get(id)
			if (t) {
				next.push(t)
				byId.delete(id)
			}
		}
		for (const t of this.#tabs) if (byId.has(t.id)) next.push(t)
		if (next.length === this.#tabs.length && next.every((t, i) => t === this.#tabs[i])) return
		this.#tabs = next
		this.#flush()
	}

	close(id: string): void {
		const idx = this.#tabs.findIndex((t) => t.id === id)
		if (idx < 0) return
		this.#tabs.splice(idx, 1)
		if (this.#activeId === id) {
			this.#activeId = (this.#tabs[idx] ?? this.#tabs[idx - 1] ?? this.#tabs[0])?.id ?? ''
		}
		this.#flush()
	}

	setCollapsed(collapsed: boolean): void {
		if (this.#collapsed === collapsed) return
		this.#collapsed = collapsed
		this.#flush()
	}

	// Feed back the location an iframe reported on load (only the page can read
	// contentWindow.location). Updates the observed `loc`, leaving `url` alone so
	// the tab doesn't reload.
	observeLocation(id: string, loc: string): void {
		const t = this.#tabs.find((x) => x.id === id)
		if (!t) return
		const canonical = canonicalizeObservedLoc(loc)
		if (t.loc === canonical) return
		t.loc = canonical
		this.#flush()
	}

	// Stamp the friendly display label for the editor tab hosting `target` (the
	// live editor knows the item's typed/auto name once its cell loads, which the
	// page can't read reactively from the runtime cell). Matched on the tab's
	// commanded `url` — the stable per-(kind,path) editor identity. Transient, so
	// no persist/flush: it's recomputed when the tab remounts.
	setEditorFriendlyLabel(target: SessionTarget, label: string | undefined): void {
		const t = this.#tabs.find((x) => isEditorTabFor(x.url, target))
		if (!t || t.friendlyLabel === label) return
		t.friendlyLabel = label
	}

	// Persist a pending write immediately, cancelling the debounce. Called on
	// page hide — a mutation inside the debounce window would otherwise be lost
	// to a reload/navigation. No-op when nothing is pending.
	flushNow(): void {
		if (this.#flushHandle === undefined) return
		clearTimeout(this.#flushHandle)
		this.#flushHandle = undefined
		this.#persistNow()
	}

	#flush(): void {
		// Prune cells promptly (cheap, synchronous) even though the durable persist
		// stays debounced — a closed tab's editor cell should be reclaimable now.
		this.#adapter.onTabsChanged?.()
		this.#schedulePersist()
	}

	#schedulePersist(): void {
		clearTimeout(this.#flushHandle)
		this.#flushHandle = setTimeout(() => {
			this.#flushHandle = undefined
			this.#persistNow()
		}, this.#flushDelay)
	}

	#persistNow(): void {
		this.#adapter.persist({
			tabs: this.#tabs.map((t) => ({ ...t })),
			activeId: this.#activeId,
			collapsed: this.#collapsed,
			previewSize: this.#previewSize
		})
	}
}

// Which tabs the `close_page` AI tool should close: every tab when `all`, else
// those whose page label or stripped path contains `match` (case-insensitive).
// Pure over a tab snapshot so the runtime handler can close by id and this stays
// unit-testable. An empty/whitespace match closes nothing (the handler reports it).
export function selectPreviewTabsToClose(
	tabs: SessionPreviewTab[],
	opts: { all: boolean; match: string | undefined }
): SessionPreviewTab[] {
	if (opts.all) return tabs.slice()
	const needle = opts.match?.trim().toLowerCase()
	if (!needle) return []
	return tabs.filter((t) => {
		const where = t.loc || t.url
		return (
			previewLocationLabel(where).toLowerCase().includes(needle) ||
			where.toLowerCase().includes(needle)
		)
	})
}

// Human-readable summary of a session's open preview tabs, for the
// `get_preview_status` AI tool. Pure over the owner's model. The "no session"
// case is the caller's (the tool handler has the session context).
export function describePreview(tabs: SessionPreviewTab[], activeId: string): string {
	if (tabs.length === 0) return 'No preview tabs are open in the side panel.'
	const lines = tabs.map((t) => {
		const where = t.loc || t.url
		const page = matchPreviewPage(where)
		const pipelineFolder = parsePipelineRoute(where)
		const route = parsePreviewItemRoute(where)
		const label = page
			? `page "${page.label}"`
			: pipelineFolder
				? `pipeline "${pipelineFolder}"`
				: route
					? `${route.raw_app ? 'raw_app' : route.kind} "${route.itemPath}"`
					: stripBase(where)
		const live = resolvePreviewTab(t.url).kind === 'editor' ? ', live editor' : ''
		const active = t.id === activeId ? ', active' : ''
		return `- ${label}${live}${active}`
	})
	return `${tabs.length} preview tab${tabs.length === 1 ? '' : 's'} open in the side panel:\n${lines.join('\n')}`
}
