import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import {
	SessionPreviewTabs,
	describePreview,
	hydratePreviewTabs,
	previewTargetForSessionTarget,
	type PreviewTabsAdapter,
	type PreviewTabsSnapshot
} from './sessionPreviewTabs.svelte'
import type { PreviewTarget } from './previewRouter'
import type { SessionPreviewTab, SessionTarget } from './sessionState.svelte'

// In-memory adapter spy: records persisted snapshots + target writes, no IDB.
function makeAdapter() {
	const persisted: PreviewTabsSnapshot[] = []
	const targets: SessionTarget[] = []
	const adapter: PreviewTabsAdapter = {
		persist: (snap) => persisted.push(snap),
		setTarget: (t) => targets.push(t)
	}
	return { adapter, persisted, targets }
}

function owner(initial: Partial<PreviewTabsSnapshot> = {}, adapter?: PreviewTabsAdapter) {
	return new SessionPreviewTabs(
		{ tabs: [], activeId: '', collapsed: false, ...initial },
		adapter ?? makeAdapter().adapter,
		// Deterministic debounce for the tests.
		0
	)
}

const scriptTarget: PreviewTarget = {
	type: 'item',
	item: { kind: 'script', path: 'u/me/foo', summary: '' }
}
const flowTarget: PreviewTarget = {
	type: 'item',
	item: { kind: 'flow', path: 'u/me/bar', summary: '' }
}
const rawAppTarget: PreviewTarget = {
	type: 'item',
	item: { kind: 'app', raw_app: true, path: 'u/me/app', summary: '' }
}
const dndAppTarget: PreviewTarget = {
	type: 'item',
	item: { kind: 'app', path: 'u/me/legacy', summary: '' }
}
const pageTarget: PreviewTarget = { type: 'page', href: '/runs', label: 'Runs' }

beforeEach(() => {
	vi.useFakeTimers()
})
afterEach(() => {
	vi.useRealTimers()
})

describe('hydratePreviewTabs', () => {
	it('uses saved tabs and a valid active id, panel open', () => {
		const tabs: SessionPreviewTab[] = [
			{ id: 'a', url: '/x', loc: '/x' },
			{ id: 'b', url: '/y', loc: '/y' }
		]
		const snap = hydratePreviewTabs({ previewTabs: tabs, activePreviewTabId: 'b' })
		expect(snap.tabs).toHaveLength(2)
		expect(snap.activeId).toBe('b')
		expect(snap.collapsed).toBe(false)
	})

	it('falls back to the first tab when the saved active id is stale', () => {
		const snap = hydratePreviewTabs({
			previewTabs: [{ id: 'a', url: '/x', loc: '/x' }],
			activePreviewTabId: 'gone'
		})
		expect(snap.activeId).toBe('a')
	})

	it('seeds a single tab on the editor target when there are no saved tabs', () => {
		const snap = hydratePreviewTabs({ target: { kind: 'script', path: 'u/me/foo' } })
		expect(snap.tabs).toEqual([
			{ id: 'session', url: '/scripts/edit/u/me/foo', loc: '/scripts/edit/u/me/foo' }
		])
		expect(snap.activeId).toBe('session')
		expect(snap.collapsed).toBe(false)
	})

	it('is empty and collapsed for a session with nothing to preview', () => {
		const snap = hydratePreviewTabs({})
		expect(snap.tabs).toEqual([])
		expect(snap.activeId).toBe('')
		expect(snap.collapsed).toBe(true)
	})

	it('honours an explicit previewCollapsed override', () => {
		expect(
			hydratePreviewTabs({ previewCollapsed: true, target: { kind: 'script', path: 'p' } })
				.collapsed
		).toBe(true)
		expect(hydratePreviewTabs({ previewCollapsed: false }).collapsed).toBe(false)
	})

	it('drops malformed saved tabs, duplicate ids and stray fields, defaulting loc to url', () => {
		const snap = hydratePreviewTabs({
			previewTabs: [
				// `pinned: true` mimics a record saved before the flag was retired.
				{ id: 'a', url: '/x', pinned: true } as unknown as SessionPreviewTab,
				{ id: '', url: '/no-id', loc: '/no-id' },
				{ id: 'b', url: '', loc: '' },
				{ id: 'a', url: '/dupe', loc: '/dupe' }
			],
			activePreviewTabId: 'a'
		})
		expect(snap.tabs).toEqual([{ id: 'a', url: '/x', loc: '/x' }])
		expect(snap.activeId).toBe('a')
	})

	it('falls back to the target seed when every saved tab is malformed', () => {
		const snap = hydratePreviewTabs({
			previewTabs: [{ id: '', url: '', loc: '' }],
			target: { kind: 'script', path: 'u/me/foo' }
		})
		expect(snap.tabs).toHaveLength(1)
		expect(snap.activeId).toBe('session')
	})
})

describe('previewTargetForSessionTarget', () => {
	it('maps raw_app to a raw app item', () => {
		expect(previewTargetForSessionTarget('raw_app', 'u/me/app')).toEqual({
			type: 'item',
			item: { kind: 'app', raw_app: true, path: 'u/me/app', summary: '' }
		})
	})
	it('maps script/flow straight through', () => {
		expect(previewTargetForSessionTarget('script', 'p')).toEqual({
			type: 'item',
			item: { kind: 'script', path: 'p', summary: '' }
		})
		expect(previewTargetForSessionTarget('flow', 'p')).toEqual({
			type: 'item',
			item: { kind: 'flow', path: 'p', summary: '' }
		})
	})
	it('returns undefined for pipeline (no full-page route)', () => {
		expect(previewTargetForSessionTarget('pipeline', 'p')).toBeUndefined()
	})
})

describe('SessionPreviewTabs.open', () => {
	it('opens an editor item, points the target at it, activates it, and reveals the panel', () => {
		const { adapter, targets } = makeAdapter()
		const o = owner({ collapsed: true }, adapter)
		const res = o.open(scriptTarget)
		expect(res.status).toBe('opened')
		expect(o.tabs).toHaveLength(1)
		expect(o.tabs[0].url).toBe('/scripts/edit/u/me/foo')
		expect(o.activeId).toBe(o.tabs[0].id)
		expect(o.collapsed).toBe(false)
		expect(targets).toEqual([{ kind: 'script', path: 'u/me/foo' }])
	})

	it('focuses the existing tab instead of duplicating when the item is already shown', () => {
		const o = owner()
		o.open(scriptTarget)
		const firstId = o.tabs[0].id
		o.select('nonexistent-noop') // no-op: not present
		const res = o.open(scriptTarget)
		expect(res.status).toBe('focused')
		expect(o.tabs).toHaveLength(1)
		expect(o.activeId).toBe(firstId)
	})

	it('opens a second tab and repoints the live editor for a different item', () => {
		const { adapter, targets } = makeAdapter()
		const o = owner({}, adapter)
		o.open(scriptTarget)
		const res = o.open(flowTarget)
		expect(res.status).toBe('opened')
		expect(o.tabs).toHaveLength(2)
		expect(targets.at(-1)).toEqual({ kind: 'flow', path: 'u/me/bar' })
	})

	it('opens a raw app via its apps_raw route', () => {
		const { adapter, targets } = makeAdapter()
		const o = owner({}, adapter)
		o.open(rawAppTarget)
		expect(o.tabs[0].url).toBe('/apps_raw/edit/u/me/app')
		expect(targets).toEqual([{ kind: 'raw_app', path: 'u/me/app' }])
	})

	it('always opens a fresh tab for a page and never sets a target', () => {
		const { adapter, targets } = makeAdapter()
		const o = owner({}, adapter)
		o.open(pageTarget)
		o.open(pageTarget)
		expect(o.tabs).toHaveLength(2)
		expect(targets).toEqual([])
	})

	it('does not set a target for a legacy drag-and-drop app', () => {
		const { adapter, targets } = makeAdapter()
		const o = owner({}, adapter)
		o.open(dndAppTarget)
		expect(targets).toEqual([])
		expect(o.tabs[0].url).toBe('/apps/edit/u/me/legacy')
	})
})

describe('SessionPreviewTabs.navigate', () => {
	it('retargets the active tab and sets the target for an editor item', () => {
		const { adapter, targets } = makeAdapter()
		const o = owner({}, adapter)
		o.open(pageTarget)
		const tabId = o.activeId
		o.navigate(flowTarget)
		expect(o.tabs).toHaveLength(1)
		expect(o.activeId).toBe(tabId)
		expect(o.tabs[0].url).toBe('/flows/edit/u/me/bar')
		expect(o.tabs[0].loc).toBe('/flows/edit/u/me/bar')
		expect(targets).toEqual([{ kind: 'flow', path: 'u/me/bar' }])
	})

	it('no-ops with no active tab', () => {
		const { adapter, targets } = makeAdapter()
		const o = owner({}, adapter)
		o.navigate(flowTarget)
		expect(o.tabs).toHaveLength(0)
		expect(targets).toEqual([])
	})

	it('retargets to a page without touching the target', () => {
		const { adapter, targets } = makeAdapter()
		const o = owner({}, adapter)
		o.open(scriptTarget)
		targets.length = 0
		o.navigate(pageTarget)
		expect(o.tabs[0].url).toBe('/runs')
		expect(targets).toEqual([])
	})

	it('focuses the tab already hosting the item instead of duplicating the editor', () => {
		const { adapter } = makeAdapter()
		const o = owner({}, adapter)
		o.open(scriptTarget)
		const editorTabId = o.activeId
		o.open(pageTarget)
		const pageTabId = o.activeId
		o.navigate(scriptTarget)
		expect(o.activeId).toBe(editorTabId)
		expect(o.tabs).toHaveLength(2)
		// The page tab must keep its own url — only focus moved.
		expect(o.tabs.find((t) => t.id === pageTabId)?.url).toBe('/runs')
	})
})

describe('SessionPreviewTabs.select / close / setCollapsed', () => {
	it('selects a tab', () => {
		const o = owner({
			tabs: [
				{ id: 'a', url: '/x', loc: '/x' },
				{ id: 'b', url: '/y', loc: '/y' }
			],
			activeId: 'a'
		})
		o.select('b')
		expect(o.activeId).toBe('b')
		expect(o.activeTab?.id).toBe('b')
	})

	it('closes a tab and picks a neighbour when the active one goes', () => {
		const o = owner({
			tabs: [
				{ id: 'a', url: '/x', loc: '/x' },
				{ id: 'b', url: '/y', loc: '/y' },
				{ id: 'c', url: '/z', loc: '/z' }
			],
			activeId: 'b'
		})
		o.close('b')
		expect(o.tabs.map((t) => t.id)).toEqual(['a', 'c'])
		expect(o.activeId).toBe('c')
	})

	it('closing the last tab empties the model', () => {
		const o = owner({
			tabs: [{ id: 'session', url: '/x', loc: '/x' }],
			activeId: 'session'
		})
		o.close('session')
		expect(o.tabs).toHaveLength(0)
		expect(o.activeId).toBe('')
	})

	it('toggles collapsed', () => {
		const o = owner({ collapsed: false })
		o.setCollapsed(true)
		expect(o.collapsed).toBe(true)
	})
})

describe('SessionPreviewTabs.observeLocation', () => {
	it('updates loc without touching url', () => {
		const o = owner({
			tabs: [{ id: 'a', url: '/scripts/edit/u/me/foo', loc: '/scripts/edit/u/me/foo' }],
			activeId: 'a'
		})
		o.observeLocation('a', '/scripts/edit/u/me/foo?tab=logs')
		expect(o.tabs[0].url).toBe('/scripts/edit/u/me/foo')
		expect(o.tabs[0].loc).toBe('/scripts/edit/u/me/foo?tab=logs')
	})
})

describe('SessionPreviewTabs persistence', () => {
	it('debounces a write-behind of the full model after mutations', () => {
		const { adapter, persisted } = makeAdapter()
		const o = owner({}, adapter)
		o.open(scriptTarget)
		o.open(flowTarget)
		expect(persisted).toHaveLength(0) // nothing flushed synchronously
		vi.runAllTimers()
		expect(persisted).toHaveLength(1) // coalesced to one write
		expect(persisted[0].tabs).toHaveLength(2)
		expect(persisted[0].activeId).toBe(o.activeId)
		expect(persisted[0].collapsed).toBe(false)
	})
})

describe('describePreview', () => {
	it('reports no tabs when there are none', () => {
		expect(describePreview([], '', undefined)).toContain('No preview tabs')
	})

	it('lists tabs, marks the active one, and flags the live editor', () => {
		const tabs: SessionPreviewTab[] = [
			{ id: 'a', url: '/scripts/edit/u/me/foo', loc: '/scripts/edit/u/me/foo' }
		]
		const out = describePreview(tabs, 'a', { kind: 'script', path: 'u/me/foo' })
		expect(out).toContain('1 preview tab')
		expect(out).toContain('script "u/me/foo"')
		expect(out).toContain('live editor')
		expect(out).toContain('active')
	})

	it('labels a known page and omits the live-editor flag when the target differs', () => {
		const tabs: SessionPreviewTab[] = [{ id: 'a', url: '/runs', loc: '/runs' }]
		const out = describePreview(tabs, 'a', { kind: 'script', path: 'u/me/foo' })
		expect(out).toContain('page "Runs"')
		expect(out).not.toContain('live editor')
	})
})
