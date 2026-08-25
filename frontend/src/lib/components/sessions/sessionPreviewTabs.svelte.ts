import { base } from '$lib/base'
import { randomUUID } from '$lib/utils/uuid'
import { editPathFor, type WorkspaceItem } from '$lib/components/workspacePicker'
import { normalizePipelineFolder } from '$lib/utils/pipelineFolder'
import {
	artifactUrl,
	canonicalizeObservedLoc,
	describeLocation,
	matchPreviewPage,
	showsView,
	parseArtifactRoute,
	parsePipelineRoute,
	previewLocationContext,
	promptSafe,
	parsePreviewItemRoute,
	previewLocationLabel,
	resolvePreviewTab,
	stripBase,
	type ArtifactVersionTarget,
	type PreviewTarget
} from './previewRouter'
import type { SessionPreviewTab, SessionTarget } from './sessionState.svelte'
import type { Kind } from '$lib/utils_deployable'
import { pipelineFolderFromBundlePath } from '$lib/pipelinePaths'

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
	// Fired when open() creates a brand-new tab (not focus/retarget of an
	// existing one), with the tab's initial URL.
	onTabOpened?: (url: string) => void
}

// True when a tab's URL is the live editor for a specific editable item. Every
// editable route resolves to an editor, so this doubles as the "same item" dedupe
// test in open()/navigate().
function isEditorTabFor(url: string, target: SessionTarget): boolean {
	const slot = resolvePreviewTab(url)
	return slot.kind === 'editor' && slot.editorKind === target.kind && slot.path === target.path
}

// The version a tab shows when it is re-pointed: the opener's, if it named one, else whatever
// pin is already on the tab. Re-pointing must never *silently* double as "show the newest" —
// every artifact tool re-opens the document it just wrote, so that would yank the reader out of
// the version they chose on each edit; an opener that does want the newest text says 'latest'.
// The pin belongs to a (tab, artifact) pair: a different document, or a brand-new tab, starts
// unpinned.
function keptVersion(
	target: { id: string; version?: ArtifactVersionTarget },
	onto: SessionPreviewTab | undefined
): number | undefined {
	if (target.version !== undefined) return target.version === 'latest' ? undefined : target.version
	const current = onto && parseArtifactRoute(onto.url)
	return current?.id === target.id ? current.version : undefined
}

// URL a tab should load for a destination: a page's href, an item's edit route, or an artifact's
// scheme. `onto` is the tab about to be written, passed wherever one is being re-pointed so
// that every such path keeps its pin.
function targetUrl(target: PreviewTarget, onto?: SessionPreviewTab): string {
	if (target.type === 'page') return target.href
	if (target.type === 'artifact') {
		return artifactUrl(target.id, target.name, keptVersion(target, onto))
	}
	return `${base}${editPathFor(target.item)}`
}

// Point a tab at a new destination. Clears `friendlyLabel`/`friendlyPath`
// (bound to the previous editor's item): a new editor re-stamps them, and
// navigating to a plain page must drop the stale name so the tab falls back
// to the location label. Only on an actual change of destination, though —
// nothing re-stamps a tab that stays on the item it already hosts, so wiping
// there would strand its label at the storage path (`…/draft_<uuid>`).
function retargetTab(tab: SessionPreviewTab, url: string): void {
	if (tab.url !== url) {
		tab.friendlyLabel = undefined
		tab.friendlyPath = undefined
		tab.editorNamed = undefined
	}
	tab.url = url
	tab.loc = url
}

// A tab carries two locations and each write touches a different one, so the choice is a
// function name rather than a judgement: `url` is what we last commanded and what the host
// loads; `loc` is where the frame actually went, written only by the observer.

/** The frame is already showing `url` — record what was asked for without moving it. A
 * refresh and a remount both reload from `url`, so leaving it behind sends the tab back to
 * wherever it started. */
function recordCommand(tab: SessionPreviewTab, url: string): void {
	tab.url = url
}

/** Where the tab is, as well as we know: the frame's own location once it has reported
 * one, else what we commanded. */
export function whereIs(tab: Pick<SessionPreviewTab, 'url' | 'loc'>): string {
	return tab.loc || tab.url
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
		const folder = normalizePipelineFolder(path)
		return { type: 'page', href: `${base}/pipeline/${encodeURIComponent(folder)}`, label: folder }
	}
	const item: WorkspaceItem =
		kind === 'raw_app'
			? { kind: 'app', raw_app: true, path, summary: '' }
			: { kind, path, summary: '' }
	return { type: 'item', item }
}

// Adapt a deployable item's layout kind (the session review dock speaks `Kind`,
// not SessionTarget) to a preview destination: the three live editors, data
// pipelines, plus legacy drag-and-drop apps, which the panel hosts as an iframe
// over their edit route. Every other kind maps to undefined — not for lack of any
// route (a variable or trigger has a list page the panel can host) but because
// there is no item editor to preview, so their row falls back to the diff. The
// undefined is also the caller's test for "can this row be previewed?".
export function previewTargetForDeployKind(kind: Kind, path: string): PreviewTarget | undefined {
	if (kind === 'app') {
		return { type: 'item', item: { kind: 'app', raw_app: false, path, summary: '' } }
	}
	if (kind === 'script' || kind === 'flow' || kind === 'raw_app') {
		return previewTargetForSessionTarget(kind, path)
	}
	// A pipeline's editor is its folder's graph view, not its bundle path.
	if (kind === 'data_pipeline') {
		const folder = pipelineFolderFromBundlePath(path)
		return folder ? previewTargetForSessionTarget('pipeline', folder) : undefined
	}
	return undefined
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
		tabs.push({ id: t.id, url: t.url, loc: t.loc || t.url })
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
	// Ephemeral UI signals — not part of the persisted snapshot.
	#focusPulse = $state({ id: '', nonce: 0 })
	#reloadPulse = $state({ id: '', nonce: 0 })
	// Set while a mutation sequence is being judged as a whole (see asOneChange).
	#pulsing = false
	// Fullscreen overrides the collapsed layout, so the panel can be on screen
	// while `#collapsed` still says otherwise. Page-level state (it outlives a
	// session switch, unlike the persisted per-session flag), pushed in here so
	// the flash decision reads what the user can actually see.
	#fullscreen = false
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
	/** The tab the user can actually see, or undefined when the panel is not on
	 * screen. Anything describing the preview to the user (or to the chat) wants
	 * this, not `activeTab` — which answers for a collapsed panel too. */
	get displayedTab(): SessionPreviewTab | undefined {
		return this.#displayedTab()
	}
	get collapsed(): boolean {
		return this.#collapsed
	}
	get previewSize(): number | undefined {
		return this.#previewSize
	}
	get focusPulse(): { id: string; nonce: number } {
		return this.#focusPulse
	}

	// The nonce makes each call a fresh value, so re-clicking the same active tab
	// still fires the flash.
	pulseFocus(id: string): void {
		this.#focusPulse = { id, nonce: this.#focusPulse.nonce + 1 }
	}

	get reloadPulse(): { id: string; nonce: number } {
		return this.#reloadPulse
	}

	// Point a tab at `url` and make sure the frame follows. The host navigates off a
	// change of the commanded `url`, so re-commanding one a tab already points at moves
	// nothing — exactly the case where `loc` shows the frame drifted elsewhere.
	#retarget(tab: SessionPreviewTab, url: string): void {
		const commandUnchanged = tab.url === url
		// Drift is a change of what the frame *shows*, not of its URL string: a page
		// writing its own filter defaults back is not the user navigating away.
		const drifted = !showsView(tab.loc, url)
		// Both cases the browser will not act on, decided here because this is where the
		// old and new commands are both in hand: re-commanding the URL a drifted frame
		// already carries moves nothing, and moving to another fragment resolves within the
		// same document — so a list page never re-runs the `#<path>` read that opens a row.
		// Dropping the fragment is not one of them: the same-document path applies only to a
		// target that has one, so the browser loads the page — closing the drawer by itself —
		// and forcing a second load races that one back onto the row.
		const fragmentOnly =
			!commandUnchanged && url.includes('#') && tab.url.split('#')[0] === url.split('#')[0]
		retargetTab(tab, url)
		if ((commandUnchanged && drifted) || fragmentOnly) this.pulseReload(tab.id)
	}

	// Force the host to reload the iframe. A navigation onto the tab's exact current URL
	// changes nothing, so URL-driven behavior — a `#<path>` opening a drawer the user has
	// since closed — would never re-fire.
	pulseReload(id: string): void {
		this.#reloadPulse = { id, nonce: this.#reloadPulse.nonce + 1 }
	}

	setPreviewSize(size: number): void {
		if (this.#previewSize === size) return
		this.#previewSize = size
		// A size change never touches the tab set, so skip the editor-cell prune
		// (onTabsChanged) and only schedule the debounced persist.
		this.#schedulePersist()
	}

	// Whether the panel is on screen at all — fullscreen wins over collapsed.
	setFullscreen(fullscreen: boolean): void {
		this.#fullscreen = fullscreen
	}

	// The tab the user is actually looking at, or undefined when nothing is on screen.
	#displayedTab(): SessionPreviewTab | undefined {
		if (this.#collapsed && !this.#fullscreen) return undefined
		return this.#tabs.find((t) => t.id === this.#activeId)
	}

	// Run a caller's own multi-call sequence (select + navigate + reveal) as a
	// single change for the flash decision. Judging each call separately would
	// flash a tab the sequence had just switched to, since the step that made it
	// visible is not the step that finds nothing left to change. `mutate` must be
	// synchronous: the verdict is read the moment it returns.
	asOneChange<T>(mutate: () => T): T {
		return this.#pulsingIfUnchanged(mutate)
	}

	// Run a tab mutation, flashing the displayed tab's border when it left the
	// panel showing exactly what it already showed. Re-opening a destination that
	// is already on screen is otherwise indistinguishable from a dead click.
	#pulsingIfUnchanged<T>(mutate: () => T): T {
		// Already inside a sequence: that outer call owns the decision, and an
		// inner open()/navigate() must not rule on its own slice of it.
		if (this.#pulsing) return mutate()
		this.#pulsing = true
		try {
			return this.#pulseIfSameDestination(mutate)
		} finally {
			this.#pulsing = false
		}
	}

	#pulseIfSameDestination<T>(mutate: () => T): T {
		const before = this.#displayedTab()
		const shown = before && { id: before.id, url: before.url, loc: before.loc }
		const result = mutate()
		const after = this.#displayedTab()
		if (
			shown &&
			after &&
			after.id === shown.id &&
			after.url === shown.url &&
			after.loc === shown.loc
		) {
			this.pulseFocus(after.id)
		}
		return result
	}

	// Open — or focus, if already shown — a tab for a destination, and reveal the
	// panel. An editable item dedupes against the tab already hosting that same
	// (kind, path); anything else dedupes on the tab's observed location.
	// `forceNewTab` opts a page out of that location dedupe (open_page's `new_tab`).
	// It deliberately does not reach the item, pipeline and artifact branches: those
	// dedupe because a second tab would fight over one piece of shared state.
	open(
		target: PreviewTarget,
		opts?: { forceNewTab?: boolean }
	): { status: 'opened' | 'focused' | 'retargeted' } {
		return this.#pulsingIfUnchanged(() => this.#open(target, opts))
	}

	#open(
		target: PreviewTarget,
		opts?: { forceNewTab?: boolean }
	): { status: 'opened' | 'focused' | 'retargeted' } {
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
				this.#retarget(existing, url)
				this.#activeId = existing.id
				this.#flush()
				return { status: same ? 'focused' : 'opened' }
			}
		}
		// Dedupe artifacts by id, not full url: an update may have changed the name the url carries.
		if (target.type === 'artifact') {
			const existing = this.#tabs.find((t) => parseArtifactRoute(t.url)?.id === target.id)
			if (existing) {
				// `same` is judged against the url actually written (which keeps the tab's pin —
				// see keptVersion), else preserving a pin would report 'opened' with nothing moved.
				const kept = targetUrl(target, existing)
				const same = existing.url === kept
				retargetTab(existing, kept)
				this.#activeId = existing.id
				this.#flush()
				return { status: same ? 'focused' : 'opened' }
			}
		}
		// Matched on the observed `loc`, not `url`: a tab that navigated away no longer
		// shows this. The tab on this exact view wins over any other on the page —
		// `new_tab` puts two views side by side, and retargeting whichever sits first
		// would overwrite the other and leave both on the same row.
		const shown = opts?.forceNewTab
			? undefined
			: (this.#tabs.find((t) => showsView(t.loc, url)) ??
				this.#tabs.find((t) => describeLocation(t.loc).identity === describeLocation(url).identity))
		if (shown) {
			const same = showsView(shown.loc, url)
			if (same) {
				// The frame is already here, but record what was asked for: `url` is what the
				// tab persists and remounts from, so leaving it on where the frame started
				// sends a refresh back to the row the user has since moved off.
				recordCommand(shown, url)
				// Nothing to navigate to, so nothing would re-run: the list pages read their
				// `#<path>` once per document, and the drawer it opens may since have been
				// closed. Only a forced load can bring it back.
				if (describeLocation(url).anchor) this.pulseReload(shown.id)
			} else {
				this.#retarget(shown, url)
			}
			this.#activeId = shown.id
			this.#flush()
			return { status: same ? 'focused' : 'retargeted' }
		}
		const tab: SessionPreviewTab = { id: randomUUID(), url, loc: url }
		this.#tabs.push(tab)
		this.#activeId = tab.id
		this.#flush()
		this.#adapter.onTabOpened?.(url)
		return { status: 'opened' }
	}

	// Re-point the active tab at a destination (breadcrumb pick / in-editor link /
	// iframe-posted editor navigation).
	navigate(target: PreviewTarget): void {
		this.#pulsingIfUnchanged(() => this.#navigate(target))
	}

	#navigate(target: PreviewTarget): void {
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
		// Keep at most one pipeline tab (all share runtime.pipelineEditorState): if a
		// *different* tab already hosts a pipeline, retarget and focus it rather than
		// turning the active tab into a second pipeline editor racing the shared
		// state. Same invariant as open(); a no-op when the active tab is that tab.
		const pipelineFolder = parsePipelineRoute(targetUrl(target))
		if (pipelineFolder) {
			const existing = this.#tabs.find((x) => parsePipelineRoute(x.url) !== null)
			if (existing && existing.id !== t.id) {
				this.#retarget(existing, targetUrl(target, existing))
				this.#activeId = existing.id
				this.#flush()
				return
			}
		}
		// Same by-id artifact dedupe as open(): focus (and re-point, in case the
		// name changed) the tab already viewing this artifact instead of turning
		// the active tab into a duplicate viewer.
		if (target.type === 'artifact') {
			const existing = this.#tabs.find((x) => parseArtifactRoute(x.url)?.id === target.id)
			if (existing && existing.id !== t.id) {
				this.#retarget(existing, targetUrl(target, existing))
				this.#activeId = existing.id
				this.#flush()
				return
			}
		}
		this.#retarget(t, targetUrl(target, t))
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

	/** Show a version of an artifact already open, or the current one when `version` is
	 * undefined. Reader-driven: re-pointing a tab preserves a pin instead (see keptVersion). */
	pinArtifactVersion(artifactId: string, version: number | undefined): void {
		const tab = this.#tabs.find((t) => parseArtifactRoute(t.url)?.id === artifactId)
		const route = tab && parseArtifactRoute(tab.url)
		if (!tab || !route) return
		const url = artifactUrl(artifactId, route.name, version)
		if (tab.url === url) return
		tab.url = url
		tab.loc = url
		this.#flush()
	}

	closeArtifact(artifactId: string): void {
		const tab = this.#tabs.find((t) => parseArtifactRoute(t.url)?.id === artifactId)
		if (tab) this.close(tab.id)
	}

	setCollapsed(collapsed: boolean): void {
		if (this.#collapsed === collapsed) return
		this.#collapsed = collapsed
		this.#flush()
	}

	// Feed back the location an iframe reported on load (only the page can read
	// contentWindow.location). Updates the observed `loc`; `url` follows only when a
	// drawer closed (below), and the host navigates on a command it isn't already at,
	// so that write does not move the frame.
	observeLocation(id: string, loc: string): void {
		const t = this.#tabs.find((x) => x.id === id)
		if (!t) return
		const canonical = canonicalizeObservedLoc(loc)
		if (t.loc === canonical) return
		t.loc = canonical
		// Closing a drawer drops the row from the frame's URL. The command has to follow, or
		// the tab reopens it on the next mount — the iframe loads `url`, not `loc`. Only the
		// anchor: any other in-frame move is the user browsing, which must not re-command.
		const commanded = describeLocation(t.url)
		const observed = describeLocation(canonical)
		if (commanded.anchor && !observed.anchor && commanded.identity === observed.identity) {
			t.url = t.url.split('#')[0]
		}
		this.#flush()
	}

	// Stamp the friendly display label (and full friendly path, which scopes the
	// breadcrumb picker) for the editor tab hosting `target` (the live editor
	// knows the item's summary / typed name once its cell loads, which the page
	// can't read reactively from the runtime cell). Matched on the tab's commanded
	// `url` — the stable per-(kind,path) editor identity. Transient, so no
	// persist/flush: they're recomputed when the tab remounts. Callers must only
	// call this once the item has loaded: it also marks the tab as named by its
	// editor, which stops the page falling back to the workspace listing.
	setEditorFriendlyLabel(
		target: SessionTarget,
		label: string | undefined,
		friendlyPath?: string
	): void {
		const t = this.#tabs.find((x) => isEditorTabFor(x.url, target))
		if (!t) return
		// Set before the no-change early return: an item with neither a summary nor
		// a staged path leaves both fields undefined, and the editor still owns it.
		t.editorNamed = true
		if (t.friendlyLabel === label && t.friendlyPath === friendlyPath) return
		t.friendlyLabel = label
		t.friendlyPath = friendlyPath
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
		const where = whereIs(t)
		return (
			previewLocationLabel(where).toLowerCase().includes(needle) ||
			where.toLowerCase().includes(needle)
		)
	})
}

// A list page's query and hash carry what the page label drops: the filters in
// force and, on the pages that deep-link one (`/schedules#u/me/daily`), the row
// whose drawer is open. Empty for a bare page, so a plain tab costs nothing.
// This string is a tool result, so it is assembled from the parts previewRouter
// recognizes rather than from the location itself: an iframe tab can host a legacy app,
// whose hash is app state, and any page can carry a filter value the user typed.
function previewLocationDetail(where: string): string {
	const { location, open } = previewLocationContext(where)
	const detail = [location === stripBase(where) ? undefined : location, open && `open: ${open}`]
		.filter(Boolean)
		.join(', ')
	return detail ? ` (${detail})` : ''
}

// Human-readable summary of a session's open preview tabs, for the
// `get_preview_status` AI tool. Pure over the owner's model. The "no session"
// case is the caller's (the tool handler has the session context).
export function describePreview(
	tabs: SessionPreviewTab[],
	activeId: string,
	onScreen: boolean = true
): string {
	if (tabs.length === 0) return 'No preview tabs are open in the side panel.'
	const lines = tabs.map((t) => {
		const where = whereIs(t)
		const artifact = parseArtifactRoute(where)
		const page = matchPreviewPage(where)
		const pipelineFolder = parsePipelineRoute(where)
		const route = parsePreviewItemRoute(where)
		const label = artifact
			? // A pinned tab is not showing what the assistant last wrote, and nothing else in this
				// summary would tell it so.
				`artifact "${artifact.name || 'Artifact'}"${artifact.version ? ` (pinned to v${artifact.version})` : ''}`
			: page
				? `page "${page.label}"${previewLocationDetail(where)}`
				: pipelineFolder
					? `pipeline "${pipelineFolder}"`
					: route
						? `${route.raw_app ? 'raw_app' : route.kind} "${route.itemPath}"`
						: // Trigger list pages land here (they're outside PREVIEW_PAGES), and
							// their `#<path>` is the trigger the drawer has open.
							`${stripBase(where)}${previewLocationDetail(where)}`
		const live = resolvePreviewTab(t.url).kind === 'editor' ? ', live editor' : ''
		const active = t.id === activeId ? ', active' : ''
		// One list entry per tab: an artifact's name, a pipeline folder and an item path
		// all arrive decoded from a URL, so any of them could otherwise write a line here.
		return `- ${promptSafe(label)}${live}${active}`
	})
	// Whether a tab is *selected* and whether the user can *see* it are different facts,
	// and both descriptions the chat receives have to agree on the second one.
	const hidden = onScreen ? '' : '\nThe side panel is collapsed, so none of these is on screen.'
	return `${tabs.length} preview tab${tabs.length === 1 ? '' : 's'} open in the side panel:\n${lines.join('\n')}${hidden}`
}
