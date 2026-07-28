import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import {
	SessionPreviewTabs,
	describePreview,
	hydratePreviewTabs,
	previewTargetForSessionTarget,
	selectPreviewTabsToClose,
	type PreviewTabsAdapter,
	type PreviewTabsSnapshot
} from './sessionPreviewTabs.svelte'
import { artifactUrl, type PreviewTarget } from './previewRouter'
import type { SessionPreviewTab } from './sessionState.svelte'
import { base } from '$lib/base'

// In-memory adapter spy: records persisted snapshots, no IDB.
function makeAdapter() {
	const persisted: PreviewTabsSnapshot[] = []
	const adapter: PreviewTabsAdapter = {
		persist: (snap) => persisted.push(snap)
	}
	return { adapter, persisted }
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
const pipelineTarget: PreviewTarget = { type: 'page', href: `${base}/pipeline/crm`, label: 'crm' }
const pipelineTarget2: PreviewTarget = {
	type: 'page',
	href: `${base}/pipeline/sales`,
	label: 'sales'
}
const artifactTarget: PreviewTarget = { type: 'artifact', id: 'art1', name: 'Plan' }

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

	it('is empty and collapsed for a session with nothing to preview', () => {
		const snap = hydratePreviewTabs({})
		expect(snap.tabs).toEqual([])
		expect(snap.activeId).toBe('')
		expect(snap.collapsed).toBe(true)
	})

	it('honours an explicit previewCollapsed override', () => {
		expect(hydratePreviewTabs({ previewCollapsed: true }).collapsed).toBe(true)
		expect(hydratePreviewTabs({ previewCollapsed: false }).collapsed).toBe(false)
	})

	it('restores the saved previewSize (with tabs and empty)', () => {
		const withTabs = hydratePreviewTabs({
			previewTabs: [{ id: 'a', url: '/x', loc: '/x' }],
			previewSize: 70
		})
		expect(withTabs.previewSize).toBe(70)
		expect(hydratePreviewTabs({ previewSize: 40 }).previewSize).toBe(40)
		expect(hydratePreviewTabs({}).previewSize).toBeUndefined()
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

	it('is empty when every saved tab is malformed', () => {
		const snap = hydratePreviewTabs({
			previewTabs: [{ id: '', url: '', loc: '' }]
		})
		expect(snap.tabs).toEqual([])
		expect(snap.activeId).toBe('')
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
	it('maps pipeline to its folder route page target', () => {
		expect(previewTargetForSessionTarget('pipeline', 'my_folder')).toEqual({
			type: 'page',
			href: `${base}/pipeline/my_folder`,
			label: 'my_folder'
		})
	})
})

describe('SessionPreviewTabs.open', () => {
	it('opens an editor item, activates it, and reveals the panel', () => {
		const o = owner({ collapsed: true })
		const res = o.open(scriptTarget)
		expect(res.status).toBe('opened')
		expect(o.tabs).toHaveLength(1)
		expect(o.tabs[0].url).toBe('/scripts/edit/u/me/foo')
		expect(o.activeId).toBe(o.tabs[0].id)
		expect(o.collapsed).toBe(false)
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

	it('opens a second tab for a different editor item', () => {
		const o = owner()
		o.open(scriptTarget)
		const res = o.open(flowTarget)
		expect(res.status).toBe('opened')
		expect(o.tabs).toHaveLength(2)
		expect(o.tabs.at(-1)!.url).toBe('/flows/edit/u/me/bar')
	})

	it('opens a raw app via its apps_raw route', () => {
		const o = owner()
		o.open(rawAppTarget)
		expect(o.tabs[0].url).toBe('/apps_raw/edit/u/me/app')
	})

	it('focuses the tab already showing a page instead of duplicating', () => {
		const o = owner()
		o.open(pageTarget)
		const firstId = o.activeId
		o.open(scriptTarget)
		const res = o.open(pageTarget)
		expect(res.status).toBe('focused')
		expect(o.tabs).toHaveLength(2)
		expect(o.activeId).toBe(firstId)
	})

	it('opens a fresh page tab when the original navigated away', () => {
		const o = owner()
		o.open(pageTarget)
		o.observeLocation(o.activeId, '/variables')
		const res = o.open(pageTarget)
		expect(res.status).toBe('opened')
		expect(o.tabs).toHaveLength(2)
	})

	it('focuses (not duplicates) a page tab whose iframe reported the injected workspace/nomenubar params', () => {
		const o = owner()
		o.open(pageTarget) // /runs
		// The iframe loads with the params the preview injects on the src.
		o.observeLocation(o.activeId, '/runs?nomenubar=true&workspace=wm-fork-x')
		const res = o.open(pageTarget)
		expect(res.status).toBe('focused')
		expect(o.tabs).toHaveLength(1)
	})

	it('opens a legacy drag-and-drop app as an iframe route', () => {
		const o = owner()
		o.open(dndAppTarget)
		expect(o.tabs[0].url).toBe('/apps/edit/u/me/legacy')
	})

	it('opens an artifact tab keyed by its synthetic url and reveals the panel', () => {
		const o = owner({ collapsed: true })
		const res = o.open(artifactTarget)
		expect(res.status).toBe('opened')
		expect(o.tabs).toHaveLength(1)
		expect(o.tabs[0].url).toBe(artifactUrl('art1', 'Plan'))
		expect(o.collapsed).toBe(false)
	})

	it('dedupes an artifact by id: re-opening focuses the same tab', () => {
		const o = owner()
		o.open(artifactTarget)
		const id = o.tabs[0].id
		const res = o.open(artifactTarget)
		expect(res.status).toBe('focused')
		expect(o.tabs).toHaveLength(1)
		expect(o.activeId).toBe(id)
	})

	it('re-points the same tab (no duplicate) when the artifact was renamed', () => {
		const o = owner()
		o.open(artifactTarget)
		const id = o.tabs[0].id
		const res = o.open({ type: 'artifact', id: 'art1', name: 'Renamed plan' })
		expect(o.tabs).toHaveLength(1)
		expect(o.tabs[0].id).toBe(id)
		expect(o.tabs[0].url).toBe(artifactUrl('art1', 'Renamed plan'))
		// URL changed (name), so the tab content differs → 'opened', not 'focused'.
		expect(res.status).toBe('opened')
	})

	it('opens separate tabs for different artifact ids', () => {
		const o = owner()
		o.open(artifactTarget)
		o.open({ type: 'artifact', id: 'art2', name: 'Other' })
		expect(o.tabs).toHaveLength(2)
	})
})

describe('SessionPreviewTabs.navigate', () => {
	it('retargets the active tab to an editor item', () => {
		const o = owner()
		o.open(pageTarget)
		const tabId = o.activeId
		o.navigate(flowTarget)
		expect(o.tabs).toHaveLength(1)
		expect(o.activeId).toBe(tabId)
		expect(o.tabs[0].url).toBe('/flows/edit/u/me/bar')
		expect(o.tabs[0].loc).toBe('/flows/edit/u/me/bar')
	})

	it('no-ops with no active tab', () => {
		const o = owner()
		o.navigate(flowTarget)
		expect(o.tabs).toHaveLength(0)
	})

	it('retargets to a page', () => {
		const o = owner()
		o.open(scriptTarget)
		o.navigate(pageTarget)
		expect(o.tabs[0].url).toBe('/runs')
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

	it('retargets the one pipeline tab instead of turning the active tab into a second', () => {
		const o = owner()
		o.open(pipelineTarget)
		const pipelineTabId = o.activeId
		o.open(scriptTarget) // a second, non-pipeline tab is now active
		const scriptTabId = o.activeId
		o.navigate(pipelineTarget2)
		// No second pipeline editor: the existing one is retargeted and focused.
		expect(o.tabs).toHaveLength(2)
		expect(o.activeId).toBe(pipelineTabId)
		expect(o.tabs.find((t) => t.id === pipelineTabId)?.url).toBe(`${base}/pipeline/sales`)
		// The script tab is untouched.
		expect(o.tabs.find((t) => t.id === scriptTabId)?.url).toBe('/scripts/edit/u/me/foo')
	})

	it('retargets the active pipeline tab in place to a new folder', () => {
		const o = owner()
		o.open(pipelineTarget)
		const tabId = o.activeId
		o.navigate(pipelineTarget2)
		expect(o.tabs).toHaveLength(1)
		expect(o.activeId).toBe(tabId)
		expect(o.tabs[0].url).toBe(`${base}/pipeline/sales`)
	})

	it('focuses the tab already viewing the artifact instead of duplicating the viewer', () => {
		const o = owner()
		o.open(artifactTarget)
		const artifactTabId = o.activeId
		o.open(pageTarget)
		const pageTabId = o.activeId
		o.navigate({ type: 'artifact', id: 'art1', name: 'Renamed plan' })
		expect(o.tabs).toHaveLength(2)
		expect(o.activeId).toBe(artifactTabId)
		// Focus moved and the viewer tab picked up the rename; the page tab kept its url.
		expect(o.tabs.find((t) => t.id === artifactTabId)?.url).toBe(
			artifactUrl('art1', 'Renamed plan')
		)
		expect(o.tabs.find((t) => t.id === pageTabId)?.url).toBe('/runs')
	})

	it('retargets the active tab in place to an artifact', () => {
		const o = owner()
		o.open(pageTarget)
		const tabId = o.activeId
		o.navigate(artifactTarget)
		expect(o.tabs).toHaveLength(1)
		expect(o.activeId).toBe(tabId)
		expect(o.tabs[0].url).toBe(artifactUrl('art1', 'Plan'))
	})

	it('drops a stale friendly label and path when the tab is retargeted', () => {
		const o = owner()
		o.open(flowTarget)
		o.setEditorFriendlyLabel(
			{ kind: 'flow', path: 'u/me/bar' },
			'luminous_flow',
			'u/me/luminous_flow'
		)
		expect(o.tabs[0].friendlyLabel).toBe('luminous_flow')
		expect(o.tabs[0].friendlyPath).toBe('u/me/luminous_flow')
		// Navigating the same tab to a plain page must clear the flow's name.
		o.navigate(pageTarget)
		expect(o.tabs[0].friendlyLabel).toBeUndefined()
		expect(o.tabs[0].friendlyPath).toBeUndefined()
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

	it('closeArtifact closes the tab showing that artifact, leaving others', () => {
		const o = owner()
		o.open(artifactTarget) // id 'art1'
		o.open(scriptTarget)
		expect(o.tabs).toHaveLength(2)
		o.closeArtifact('art1')
		expect(o.tabs.map((t) => t.url)).toEqual(['/scripts/edit/u/me/foo'])
	})

	it('closeArtifact is a no-op for an unknown artifact id', () => {
		const o = owner()
		o.open(artifactTarget)
		o.closeArtifact('nope')
		expect(o.tabs).toHaveLength(1)
	})

	it('toggles collapsed', () => {
		const o = owner({ collapsed: false })
		o.setCollapsed(true)
		expect(o.collapsed).toBe(true)
	})

	it('sets previewSize and flushes it into the snapshot', () => {
		const { adapter, persisted } = makeAdapter()
		const o = owner({ previewSize: 50 }, adapter)
		o.setPreviewSize(70)
		expect(o.previewSize).toBe(70)
		vi.runAllTimers()
		expect(persisted.at(-1)?.previewSize).toBe(70)
	})

	it('setPreviewSize dedupes an unchanged value (no persist)', () => {
		const { adapter, persisted } = makeAdapter()
		const o = owner({ previewSize: 70 }, adapter)
		o.setPreviewSize(70)
		vi.runAllTimers()
		expect(persisted).toHaveLength(0)
	})

	it('a never-resized owner persists previewSize as undefined, never a default', () => {
		const { adapter, persisted } = makeAdapter()
		const o = owner({}, adapter) // no previewSize
		o.open(scriptTarget) // any tab mutation triggers a flush
		vi.runAllTimers()
		expect(persisted.at(-1)?.previewSize).toBeUndefined()
	})

	it('setPreviewSize skips the tab-cell prune (onTabsChanged)', () => {
		const onTabsChanged = vi.fn()
		const o = owner({ previewSize: 50 }, { persist: () => {}, onTabsChanged })
		o.setPreviewSize(70)
		vi.runAllTimers()
		expect(onTabsChanged).not.toHaveBeenCalled()
	})

	it('reset replaces the whole model and reveals the panel', () => {
		const { adapter, persisted } = makeAdapter()
		const o = owner(
			{
				tabs: [
					{ id: 'a', url: '/x', loc: '/x' },
					{ id: 'b', url: '/y', loc: '/y' }
				],
				activeId: 'b',
				collapsed: true
			},
			adapter
		)
		o.reset([{ id: 'session', url: '/z', loc: '/z' }], 'session')
		expect(o.tabs.map((t) => t.id)).toEqual(['session'])
		expect(o.activeId).toBe('session')
		expect(o.collapsed).toBe(false)
		vi.runAllTimers()
		expect(persisted.at(-1)?.tabs.map((t) => t.url)).toEqual(['/z'])
	})
})

describe('SessionPreviewTabs.reorder', () => {
	it('reorders tabs to the given id order and persists, keeping the active id', () => {
		const { adapter, persisted } = makeAdapter()
		const o = owner(
			{
				tabs: [
					{ id: 'a', url: '/x', loc: '/x' },
					{ id: 'b', url: '/y', loc: '/y' },
					{ id: 'c', url: '/z', loc: '/z' }
				],
				activeId: 'a'
			},
			adapter
		)
		o.reorder(['c', 'a', 'b'])
		expect(o.tabs.map((t) => t.id)).toEqual(['c', 'a', 'b'])
		expect(o.activeId).toBe('a')
		vi.runAllTimers()
		expect(persisted.at(-1)?.tabs.map((t) => t.id)).toEqual(['c', 'a', 'b'])
	})

	it('ignores unknown ids and keeps omitted tabs at the end', () => {
		const o = owner({
			tabs: [
				{ id: 'a', url: '/x', loc: '/x' },
				{ id: 'b', url: '/y', loc: '/y' },
				{ id: 'c', url: '/z', loc: '/z' }
			],
			activeId: 'a'
		})
		// 'zzz' doesn't exist (ignored); 'c' omitted from the order (kept at the end).
		o.reorder(['b', 'zzz', 'a'])
		expect(o.tabs.map((t) => t.id)).toEqual(['b', 'a', 'c'])
	})

	it('is a no-op (no persist) when the order is unchanged', () => {
		const { adapter, persisted } = makeAdapter()
		const o = owner(
			{
				tabs: [
					{ id: 'a', url: '/x', loc: '/x' },
					{ id: 'b', url: '/y', loc: '/y' }
				],
				activeId: 'a'
			},
			adapter
		)
		o.reorder(['a', 'b'])
		vi.runAllTimers()
		expect(persisted).toHaveLength(0)
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

	it('flushNow persists a pending write immediately and cancels the debounce', () => {
		const { adapter, persisted } = makeAdapter()
		const o = owner({}, adapter)
		o.open(scriptTarget)
		o.flushNow()
		expect(persisted).toHaveLength(1)
		vi.runAllTimers()
		expect(persisted).toHaveLength(1) // debounce cancelled, no second write
	})

	it('flushNow is a no-op when nothing is pending', () => {
		const { adapter, persisted } = makeAdapter()
		const o = owner({}, adapter)
		o.flushNow()
		expect(persisted).toHaveLength(0)
	})
})

describe('describePreview', () => {
	it('reports no tabs when there are none', () => {
		expect(describePreview([], '')).toContain('No preview tabs')
	})

	it('lists tabs, marks the active one, and flags the live editor', () => {
		const tabs: SessionPreviewTab[] = [
			{ id: 'a', url: '/scripts/edit/u/me/foo', loc: '/scripts/edit/u/me/foo' }
		]
		const out = describePreview(tabs, 'a')
		expect(out).toContain('1 preview tab')
		expect(out).toContain('script "u/me/foo"')
		expect(out).toContain('live editor')
		expect(out).toContain('active')
	})

	it('labels a known page and omits the live-editor flag for a non-item page', () => {
		const tabs: SessionPreviewTab[] = [{ id: 'a', url: '/runs', loc: '/runs' }]
		const out = describePreview(tabs, 'a')
		expect(out).toContain('page "Runs"')
		expect(out).not.toContain('live editor')
	})

	it('labels an artifact tab by name, not the raw artifact url', () => {
		const url = artifactUrl('uuid-1', 'My Plan')
		const out = describePreview([{ id: 'a', url, loc: url }], 'a')
		expect(out).toContain('artifact "My Plan"')
		expect(out).not.toContain('artifact:uuid-1')
		expect(out).not.toContain('live editor')
	})
})

describe('selectPreviewTabsToClose', () => {
	const tabs: SessionPreviewTab[] = [
		{ id: 'runs', url: '/runs?status=failure', loc: '/runs?status=failure' },
		{ id: 'sched', url: '/schedules', loc: '/schedules' },
		{ id: 'script', url: '/scripts/edit/u/me/foo', loc: '/scripts/edit/u/me/foo' }
	]

	it('closes every tab when `all`', () => {
		expect(
			selectPreviewTabsToClose(tabs, { all: true, match: undefined }).map((t) => t.id)
		).toEqual(['runs', 'sched', 'script'])
	})

	it('matches a page by its label, case-insensitively', () => {
		expect(selectPreviewTabsToClose(tabs, { all: false, match: 'Runs' }).map((t) => t.id)).toEqual([
			'runs'
		])
	})

	it('matches an item tab by its path fragment', () => {
		expect(
			selectPreviewTabsToClose(tabs, { all: false, match: 'u/me/foo' }).map((t) => t.id)
		).toEqual(['script'])
	})

	it('closes nothing for an empty/whitespace match or no match', () => {
		expect(selectPreviewTabsToClose(tabs, { all: false, match: '   ' })).toEqual([])
		expect(selectPreviewTabsToClose(tabs, { all: false, match: 'nonexistent' })).toEqual([])
	})
})

describe('SessionPreviewTabs.pulseFocus', () => {
	it('sets the id and advances the nonce, re-firing for the same id', () => {
		const o = owner()
		expect(o.focusPulse).toEqual({ id: '', nonce: 0 })
		o.pulseFocus('tab-a')
		expect(o.focusPulse).toEqual({ id: 'tab-a', nonce: 1 })
		o.pulseFocus('tab-a')
		expect(o.focusPulse).toEqual({ id: 'tab-a', nonce: 2 })
		o.pulseFocus('tab-b')
		expect(o.focusPulse).toEqual({ id: 'tab-b', nonce: 3 })
	})
})
