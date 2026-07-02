import { base } from '$lib/base'
import { randomUUID } from '$lib/utils/uuid'
import { editPathFor, type WorkspaceItem } from '$lib/components/workspacePicker'
import {
	matchPreviewPage,
	parsePreviewItemRoute,
	resolvePreviewTab,
	stripBase,
	type PreviewTarget
} from './previewRouter'
import { sessionTargetHref } from './sessionMode.svelte'
import type { SessionPreviewTab, SessionTarget } from './sessionState.svelte'

// The single live owner of a session's preview tabs. Runs behind a small
// interface both the sessions page (renderer) and the `open_preview` tool cross,
// so there is exactly one live copy of the tab model instead of three drifting
// ones synced by effects. Persistence and the session-record `target` write are
// injected as an adapter, so the class is pure runes with no sessionState / IDB
// coupling (mirrors PipelineEditorState). Held on SessionRuntime.previewTabs.

export type PreviewTabsSnapshot = {
	tabs: SessionPreviewTab[]
	activeId: string
	collapsed: boolean
}

export type PreviewTabsAdapter = {
	// Write-behind the full tab model onto the durable backing (debounced by the
	// owner). Fire-and-forget.
	persist: (snapshot: PreviewTabsSnapshot) => void
	// Point the session's live editor at `target`. Called atomically with the tab
	// open/navigate that shows the item, killing the old url/target drift.
	setTarget: (target: SessionTarget) => void
}

// URL a tab should load for a destination: a page's href, or an item's edit route.
function targetUrl(target: PreviewTarget): string {
	return target.type === 'page' ? target.href : `${base}${editPathFor(target.item)}`
}

// The editor target a destination maps to, or undefined when it isn't an item we
// host live (static pages, legacy drag-and-drop apps). Drives the "set the
// session target iff the destination is an editable item" rule.
function editorTargetFor(target: PreviewTarget): SessionTarget | undefined {
	if (target.type !== 'item') return undefined
	const item = target.item
	if (item.kind === 'script') return { kind: 'script', path: item.path }
	if (item.kind === 'flow') return { kind: 'flow', path: item.path }
	if (item.kind === 'app' && item.raw_app) return { kind: 'raw_app', path: item.path }
	return undefined
}

// Adapt a session editor target (`open_preview` tool arg) to a preview
// destination. Pipeline targets have no full-page route, so they can't be
// previewed as a tab (returns undefined).
export function previewTargetForSessionTarget(
	kind: SessionTarget['kind'],
	path: string
): PreviewTarget | undefined {
	if (kind === 'pipeline') return undefined
	const item: WorkspaceItem =
		kind === 'raw_app'
			? { kind: 'app', raw_app: true, path, summary: '' }
			: { kind, path, summary: '' }
	return { type: 'item', item }
}

// Build the initial tab model for a session: its saved tabs, else a single tab
// on its editor target, else empty. Default collapse follows the old page
// seed rule (collapsed only for a session with nothing to preview).
export function hydratePreviewTabs(session: {
	previewTabs?: SessionPreviewTab[]
	activePreviewTabId?: string
	previewCollapsed?: boolean
	target?: SessionTarget
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
		return { tabs, activeId, collapsed: session.previewCollapsed ?? false }
	}
	const seedUrl = sessionTargetHref(session.target)
	if (seedUrl) {
		return {
			tabs: [{ id: 'session', url: seedUrl, loc: seedUrl }],
			activeId: 'session',
			collapsed: session.previewCollapsed ?? false
		}
	}
	return { tabs: [], activeId: '', collapsed: session.previewCollapsed ?? true }
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

	// Open — or focus, if already shown — a tab for a destination, and reveal the
	// panel. An editable item is made the session's live editor (setTarget) and
	// deduped against the tab already hosting it; pages always open a fresh tab.
	open(target: PreviewTarget): { status: 'opened' | 'focused' } {
		const editorTarget = editorTargetFor(target)
		if (editorTarget) this.#adapter.setTarget(editorTarget)
		// A fresh session starts collapsed, so without this the tab opens behind a
		// collapsed panel and the user sees nothing change.
		this.#collapsed = false
		if (editorTarget) {
			// resolvePreviewTab(url, target) is 'editor' exactly for the tab showing
			// `target`'s item, so it doubles as the dedupe test.
			const existing = this.#tabs.find(
				(t) => resolvePreviewTab(t.url, editorTarget).kind === 'editor'
			)
			if (existing) {
				this.#activeId = existing.id
				this.#flush()
				return { status: 'focused' }
			}
		}
		const url = targetUrl(target)
		const tab: SessionPreviewTab = { id: randomUUID(), url, loc: url }
		this.#tabs.push(tab)
		this.#activeId = tab.id
		this.#flush()
		return { status: 'opened' }
	}

	// Re-point the active tab at a destination (breadcrumb pick / in-editor link /
	// iframe-posted editor navigation). Same target rule as open: an editable item
	// becomes the session's live editor.
	navigate(target: PreviewTarget): void {
		const t = this.#tabs.find((x) => x.id === this.#activeId)
		if (!t) return
		const editorTarget = editorTargetFor(target)
		if (editorTarget) {
			this.#adapter.setTarget(editorTarget)
			// Same dedupe as open(): if another tab already hosts `target` as the
			// live editor, focus it instead of re-pointing this one — two tabs
			// resolving 'editor' for one target would mount two editor instances
			// on the same runtime slot.
			const existing = this.#tabs.find(
				(x) => resolvePreviewTab(x.url, editorTarget).kind === 'editor'
			)
			if (existing && existing.id !== t.id) {
				this.#activeId = existing.id
				this.#flush()
				return
			}
		}
		const url = targetUrl(target)
		t.url = url
		t.loc = url
		this.#flush()
	}

	select(id: string): void {
		if (this.#activeId === id) return
		this.#activeId = id
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
		if (!t || t.loc === loc) return
		t.loc = loc
		this.#flush()
	}

	#flush(): void {
		clearTimeout(this.#flushHandle)
		this.#flushHandle = setTimeout(() => {
			this.#adapter.persist({
				tabs: this.#tabs.map((t) => ({ ...t })),
				activeId: this.#activeId,
				collapsed: this.#collapsed
			})
		}, this.#flushDelay)
	}
}

// Human-readable summary of a session's open preview tabs, for the
// `get_preview_status` AI tool. Pure over the owner's model + the session target
// so the owner needs no target-read dependency. The "no session" case is the
// caller's (the tool handler has the session context).
export function describePreview(
	tabs: SessionPreviewTab[],
	activeId: string,
	target: SessionTarget | undefined
): string {
	if (tabs.length === 0) return 'No preview tabs are open in the side panel.'
	const lines = tabs.map((t) => {
		const where = t.loc || t.url
		const page = matchPreviewPage(where)
		const route = parsePreviewItemRoute(where)
		const label = page
			? `page "${page.label}"`
			: route
				? `${route.raw_app ? 'raw_app' : route.kind} "${route.itemPath}"`
				: stripBase(where)
		const live = resolvePreviewTab(t.url, target).kind === 'editor' ? ', live editor' : ''
		const active = t.id === activeId ? ', active' : ''
		return `- ${label}${live}${active}`
	})
	return `${tabs.length} preview tab${tabs.length === 1 ? '' : 's'} open in the side panel:\n${lines.join('\n')}`
}
