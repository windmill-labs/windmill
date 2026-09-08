import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import {
	SessionPreviewTabs,
	describePreview,
	hydratePreviewTabs,
	previewTargetForDeployKind,
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
	it('maps a pipeline owner path to the same folder target as the bare name', () => {
		// open_preview is routinely called with `f/<folder>`; keeping the prefix
		// would scope the editor to the folder "f/<folder>" and make every node
		// path `f/f/<folder>/…`.
		expect(previewTargetForSessionTarget('pipeline', 'f/my_folder')).toEqual({
			type: 'page',
			href: `${base}/pipeline/my_folder`,
			label: 'my_folder'
		})
	})
})

describe('previewTargetForDeployKind', () => {
	it('maps a legacy drag-and-drop app to its non-raw edit route', () => {
		expect(previewTargetForDeployKind('app', 'u/me/app')).toEqual({
			type: 'item',
			item: { kind: 'app', raw_app: false, path: 'u/me/app', summary: '' }
		})
	})
	it('routes a pipeline bundle on its folder, not its storage key', () => {
		expect(previewTargetForDeployKind('data_pipeline', 'f/crm/data_pipeline')).toEqual(
			pipelineTarget
		)
	})
	it('has no destination for kinds the preview panel cannot host', () => {
		expect(previewTargetForDeployKind('schedule', 'u/me/s')).toBeUndefined()
		expect(previewTargetForDeployKind('http_trigger', 'u/me/t')).toBeUndefined()
		expect(previewTargetForDeployKind('variable', 'u/me/v')).toBeUndefined()
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

	// A trigger list page is not a `matchReusablePage`, so the runtime's
	// navigate-in-place path doesn't cover it: re-pointing the tab has to happen
	// here or the panel keeps showing the previously opened row.
	it('re-points a page tab whose hash target changed instead of only focusing it', () => {
		const o = owner()
		const routes = (href: string) => ({ type: 'page' as const, href, label: 'HTTP routes' })
		o.open(routes('/routes#u/me/a'))
		const firstId = o.activeId

		// 'retargeted', not 'opened': the tab count is unchanged, and the caller
		// reports that to the model.
		const res = o.open(routes('/routes#u/me/b'))
		expect(res.status).toBe('retargeted')
		expect(o.tabs).toHaveLength(1)
		expect(o.activeId).toBe(firstId)
		expect(o.tabs[0].url).toBe('/routes#u/me/b')

		// Back to the bare list: still the same tab, no longer anchored at a row.
		expect(o.open(routes('/routes')).status).toBe('retargeted')
		expect(o.tabs).toHaveLength(1)
		expect(o.tabs[0].url).toBe('/routes')

		// ...and asking for the view it already shows is a plain focus.
		expect(o.open(routes('/routes')).status).toBe('focused')
	})

	// The list pages rewrite their own filter defaults into the URL after mount,
	// and `loc` follows that rewrite. Matching on anything but the path made a tab
	// stop recognizing itself, so every later open spawned a duplicate.
	it('still recognizes a tab after the page rewrote its own filter params', () => {
		const o = owner()
		const routes = (href: string) => ({ type: 'page' as const, href, label: 'HTTP routes' })
		o.open(routes('/routes#u/me/a'))
		const id = o.tabs[0].id
		o.observeLocation(id, '/routes?filter_path_of=trigger#u/me/a')

		const res = o.open(routes('/routes#u/me/b'))
		expect(res.status).toBe('retargeted')
		expect(o.tabs).toHaveLength(1)
		expect(o.tabs[0].url).toBe('/routes#u/me/b')
	})

	// `new_tab` deliberately keeps two views of one page side by side. Reopening one of
	// them must focus the tab already showing it, not retarget whichever tab happens to
	// sit first in the strip — that would overwrite the other view and leave two tabs
	// on the same row.
	it('focuses the tab already showing the exact location before retargeting by path', () => {
		const o = owner()
		const routes = (href: string) => ({ type: 'page' as const, href, label: 'HTTP routes' })
		o.open(routes('/routes#u/me/a'))
		const first = o.tabs[0].id
		o.open(routes('/routes#u/me/b'), { forceNewTab: true })
		const second = o.tabs[1].id

		expect(o.open(routes('/routes#u/me/b')).status).toBe('focused')
		expect(o.activeId).toBe(second)
		expect(o.tabs).toHaveLength(2)
		expect(o.tabs.find((t) => t.id === first)?.url).toBe('/routes#u/me/a')
	})

	// The list pages read their `#<path>` once per document, so a drawer the user closed
	// inside the frame only comes back on a forced load — and re-commanding the location
	// the tab already shows produces no navigation the host could act on.
	it('forces a load when the requested row is the one the tab already shows', () => {
		const o = owner()
		const routes = (href: string) => ({ type: 'page' as const, href, label: 'HTTP routes' })
		o.open(routes('/routes#u/me/a'))
		const id = o.tabs[0].id
		o.observeLocation(id, '/routes?filter_path_of=trigger#u/me/a')
		const before = o.reloadPulse.nonce

		expect(o.open(routes('/routes#u/me/a')).status).toBe('focused')
		expect(o.reloadPulse).toEqual({ id, nonce: before + 1 })
	})

	// Dropping the fragment is a load in itself, so the forced one lands on top of a
	// navigation still in flight — and reloads the row the command asked to leave.
	it('does not force a load when the requested location drops the row', () => {
		const o = owner()
		const routes = (href: string) => ({ type: 'page' as const, href, label: 'HTTP routes' })
		o.open(routes('/routes#u/me/a'))
		const id = o.tabs[0].id
		const before = o.reloadPulse.nonce

		o.navigate(routes('/routes'))
		expect(o.tabs.find((t) => t.id === id)?.url).toBe('/routes')
		expect(o.reloadPulse.nonce).toBe(before)
	})

	// Runs restores the user's "hide schedules" preference into the URL whenever a load
	// says nothing about it. Counting that as drift reloaded the tab on every re-open —
	// onto a page that seeds it straight back, so the only effect was the lost scroll.
	it('does not reload when the page restored a filter the request never mentioned', () => {
		const o = owner()
		const runs = () => ({ type: 'page' as const, href: '/runs', label: 'Runs' })
		o.open(runs())
		const id = o.tabs[0].id
		o.observeLocation(id, '/runs?job_trigger_kind=!schedule')
		const before = o.reloadPulse.nonce

		o.navigate(runs())
		expect(o.reloadPulse.nonce).toBe(before)
	})

	// ...but a real in-frame move away from the commanded view still is drift.
	it('reloads when the frame moved to a different view of the page', () => {
		const o = owner()
		const runs = () => ({ type: 'page' as const, href: '/runs?path=u/me/a', label: 'Runs' })
		o.open(runs())
		const id = o.tabs[0].id
		o.observeLocation(id, '/runs?path=u/me/b')
		const before = o.reloadPulse.nonce

		o.navigate(runs())
		expect(o.reloadPulse).toEqual({ id, nonce: before + 1 })
	})

	// A legacy app owns its own hash (the editor reads it as `context.hash`), so the
	// observer records app state into `loc`. Reading that as a drawer anchor would
	// retarget on reopen, and a same-document retarget forces a reload that discards
	// the state the user was looking at.
	it('focuses a legacy app whose own hash changed instead of reloading it', () => {
		const o = owner()
		const app = () => ({ type: 'page' as const, href: '/apps/edit/u/me/dash', label: 'dash' })
		o.open(app())
		const id = o.tabs[0].id
		o.observeLocation(id, '/apps/edit/u/me/dash#tab=2')

		expect(o.open(app()).status).toBe('focused')
		expect(o.tabs).toHaveLength(1)
		expect(o.tabs[0].url).toBe('/apps/edit/u/me/dash')
	})

	// Re-commanding the URL a tab is already pointed at changes nothing the host can
	// see, so the frame would stay wherever the user navigated it inside the page.
	it('forces a reload when the request matches the command but the frame drifted', () => {
		const o = owner()
		const routes = (href: string) => ({ type: 'page' as const, href, label: 'HTTP routes' })
		o.open(routes('/routes#u/me/a'))
		const id = o.tabs[0].id
		// The user clicked another trigger inside the iframe.
		o.observeLocation(id, '/routes#u/me/b')
		const before = o.reloadPulse.nonce

		const res = o.open(routes('/routes#u/me/a'))
		expect(res.status).toBe('retargeted')
		expect(o.tabs).toHaveLength(1)
		expect(o.tabs[0].loc).toBe('/routes#u/me/a')
		expect(o.reloadPulse.nonce).toBe(before + 1)
	})

	it('forceNewTab opts a page out of the location dedupe', () => {
		const o = owner()
		const routes = (href: string) => ({ type: 'page' as const, href, label: 'HTTP routes' })
		o.open(routes('/routes#u/me/a'))
		const res = o.open(routes('/routes#u/me/b'), { forceNewTab: true })
		expect(res.status).toBe('opened')
		expect(o.tabs).toHaveLength(2)
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

	it('keeps the version a reader pinned across re-opens, until they move or clear it', () => {
		const { adapter, persisted } = makeAdapter()
		const o = owner({}, adapter)
		o.open(artifactTarget)
		vi.runAllTimers()
		o.pinArtifactVersion('art1', 1)
		expect(o.tabs[0].url).toBe(artifactUrl('art1', 'Plan', 1))
		// The pin only survives a reload if it reached the durable snapshot.
		vi.runAllTimers()
		expect(persisted.at(-1)?.tabs.map((t) => t.url)).toEqual([artifactUrl('art1', 'Plan', 1)])

		// Nothing moved, so re-opening the pinned artifact is a focus, not an open.
		expect(o.open(artifactTarget).status).toBe('focused')
		expect(o.tabs[0].url).toBe(artifactUrl('art1', 'Plan', 1))

		// The re-open every artifact tool does after writing, here also carrying a rename.
		o.open({ type: 'artifact', id: 'art1', name: 'Plan, revised' })
		expect(o.tabs[0].url).toBe(artifactUrl('art1', 'Plan, revised', 1))

		o.pinArtifactVersion('art1', 2)
		expect(o.tabs[0].url).toBe(artifactUrl('art1', 'Plan, revised', 2))
		o.pinArtifactVersion('art1', undefined)
		expect(o.tabs[0].url).toBe(artifactUrl('art1', 'Plan, revised'))
	})

	it('moves the pin for an opener that names a version, and clears it for "latest"', () => {
		const o = owner()
		o.open(artifactTarget)
		o.pinArtifactVersion('art1', 1)

		// A plan card opening the version it proposed, which the reader is not on.
		o.open({ type: 'artifact', id: 'art1', name: 'Plan', version: 2 })
		expect(o.tabs[0].url).toBe(artifactUrl('art1', 'Plan', 2))

		// 'latest' is the intent omitting a version cannot express: a plan going up for
		// approval has to put the current text on screen even over a pin.
		o.open({ type: 'artifact', id: 'art1', name: 'Plan', version: 'latest' })
		expect(o.tabs[0].url).toBe(artifactUrl('art1', 'Plan'))
	})

	it('opens separate tabs for different artifact ids', () => {
		const o = owner()
		o.open(artifactTarget)
		o.open({ type: 'artifact', id: 'art2', name: 'Other' })
		expect(o.tabs).toHaveLength(2)
	})
})

describe('SessionPreviewTabs.open — commanded url', () => {
	it('records the requested row even when the frame is already showing it', () => {
		const o = owner()
		o.open({ type: 'page', href: '/routes#u/me/a', label: 'R' })
		// The user moves to another row inside the frame.
		o.observeLocation(o.tabs[0].id, '/routes#u/me/b')
		o.open({ type: 'page', href: '/routes#u/me/b', label: 'R' })
		// `url` is what a refresh and a remount reload from, so it has to follow.
		expect(o.tabs[0].url).toBe('/routes#u/me/b')
		expect(o.tabs).toHaveLength(1)
	})
})

describe('SessionPreviewTabs.observeLocation', () => {
	it('drops the row from the command when the frame closes its drawer', () => {
		const o = owner()
		o.open({ type: 'page', href: '/routes#u/me/a', label: 'R' })
		// The page clears its own hash when the drawer closes.
		o.observeLocation(o.tabs[0].id, '/routes?filter_path_of=trigger')
		// The iframe mounts from `url`, so a remount would otherwise reopen the drawer.
		expect(o.tabs[0].url).toBe('/routes')
	})

	it('leaves the command alone when the user just browses inside the frame', () => {
		const o = owner()
		o.open({ type: 'page', href: '/routes#u/me/a', label: 'R' })
		o.observeLocation(o.tabs[0].id, '/routes#u/me/b')
		expect(o.tabs[0].url).toBe('/routes#u/me/a')
	})
})

describe('SessionPreviewTabs.open — forced loads', () => {
	it('pulses when only the fragment changes, since the browser would not load', () => {
		const o = owner()
		o.open({ type: 'page', href: '/routes#u/me/a', label: 'R' })
		const before = o.reloadPulse.nonce
		o.open({ type: 'page', href: '/routes#u/me/b', label: 'R' })
		// Same document: the browser resolves the new fragment without a load, so the
		// list page never re-runs the `#<path>` read that opens the row.
		expect(o.reloadPulse.nonce).toBeGreaterThan(before)
		expect(o.tabs).toHaveLength(1)
	})

	it('does not pulse when the document itself changes', () => {
		const o = owner()
		o.open({ type: 'page', href: '/routes#u/me/a', label: 'R' })
		const before = o.reloadPulse.nonce
		o.open({ type: 'page', href: '/schedules#u/me/a', label: 'S' })
		// Different page: src changes, the browser loads it, nothing to force.
		expect(o.reloadPulse.nonce).toBe(before)
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

	// The breadcrumb picker opens highlighting the artifact the active tab already shows, so
	// re-picking it is the modal interaction — and it must not double as a reset to latest.
	it('keeps a pinned version when the breadcrumb re-points a tab to the artifact it shows', () => {
		const o = owner()
		o.open(artifactTarget)
		o.pinArtifactVersion('art1', 1)
		const artifactTabId = o.activeId

		// Active tab *is* the artifact tab: the picker's own highlight leads here.
		o.navigate({ type: 'artifact', id: 'art1', name: 'Plan' })
		expect(o.tabs[0].url).toBe(artifactUrl('art1', 'Plan', 1))

		// Same rule from another tab, where navigate() re-points and focuses this one instead.
		o.open(pageTarget)
		o.navigate({ type: 'artifact', id: 'art1', name: 'Renamed plan' })
		expect(o.activeId).toBe(artifactTabId)
		expect(o.tabs.find((t) => t.id === artifactTabId)?.url).toBe(
			artifactUrl('art1', 'Renamed plan', 1)
		)
	})

	it('carries no pin across when a tab is retargeted to a different artifact', () => {
		const o = owner()
		o.open(artifactTarget)
		o.pinArtifactVersion('art1', 1)
		o.navigate({ type: 'artifact', id: 'art2', name: 'Other' })
		expect(o.tabs[0].url).toBe(artifactUrl('art2', 'Other'))
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
		expect(o.tabs[0].editorNamed).toBeUndefined()
	})

	it('claims the tab for its editor even when the editor names nothing', () => {
		const o = owner()
		o.open(flowTarget)
		// A deployed item with no summary reports neither a label nor a staged path
		// — the same values a never-stamped tab already holds. It must still count
		// as named, or the sessions page keeps falling back to the workspace
		// listing and resurrects a summary the user just cleared.
		o.setEditorFriendlyLabel({ kind: 'flow', path: 'u/me/bar' }, undefined, undefined)
		expect(o.tabs[0].editorNamed).toBe(true)
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

	// What the chat is told the user is looking at comes from `displayedTab`, so a
	// collapsed panel must report nothing — otherwise a bare "disable it" resolves
	// against a row that is not on screen.
	it('displays no tab while collapsed, and the active one again in fullscreen', () => {
		const o = owner()
		o.open(pageTarget)
		const id = o.tabs[0].id
		expect(o.displayedTab?.id).toBe(id)

		o.setCollapsed(true)
		expect(o.displayedTab).toBeUndefined()
		expect(o.activeTab?.id).toBe(id)

		o.setFullscreen(true)
		expect(o.displayedTab?.id).toBe(id)
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

	it('describes a location by what is recognized, never passing it through whole', () => {
		const at = (loc: string, url = loc.split(/[?#]/)[0]): SessionPreviewTab[] => [
			{ id: 'a', url, loc }
		]
		// A declared filter and an anchored row are worth telling the model.
		expect(describePreview(at('/runs?path=u/me/a'), 'a')).toContain('/runs?path=u%2Fme%2Fa')
		expect(describePreview(at('/schedules#u/me/daily'), 'a')).toContain('open: u/me/daily')
		// A legacy app's hash is app state and an undeclared param is unknown text; this
		// string is a tool result, so neither may ride along.
		expect(describePreview(at('/apps/get/u/me/dash#token=sk-secret'), 'a')).not.toContain('sk-')
		expect(describePreview(at('/runs?unknown=sk-secret'), 'a')).not.toContain('sk-')
	})

	it('cannot let one tab forge a second entry in the list', () => {
		// The listing is one `- ` line per tab, and an artifact name, a pipeline folder and
		// an item path all arrive decoded from a URL.
		const forged = describePreview(
			[{ id: 'a', url: '/scripts/edit/u/me/x', loc: '/scripts/edit/u%2Fme%2Fx%0A- page "Runs"' }],
			'a'
		)
		expect(forged.split('\n')).toHaveLength(2)
		expect(
			describePreview(
				[
					{
						id: 'a',
						url: artifactUrl('i', 'N\n- page "Runs"'),
						loc: artifactUrl('i', 'N\n- page "Runs"')
					}
				],
				'a'
			).split('\n')
		).toHaveLength(2)
	})

	it('agrees with the active-preview block about what is on screen', () => {
		// A collapsed panel yields no ACTIVE PREVIEW, so this description must not call a
		// tab visible either — the chat would otherwise be told both at once.
		const tabs: SessionPreviewTab[] = [{ id: 'a', url: '/runs', loc: '/runs' }]
		expect(describePreview(tabs, 'a', true)).not.toContain('collapsed')
		const collapsed = describePreview(tabs, 'a', false)
		expect(collapsed).toContain('none of these is on screen')
	})

	it('names the row a page tab has open', () => {
		const tabs: SessionPreviewTab[] = [
			{ id: 'a', url: '/schedules', loc: '/schedules#u/me/daily_report' }
		]
		expect(describePreview(tabs, 'a')).toContain('page "Schedules" (open: u/me/daily_report)')
	})

	it('reports the pinned version, so the assistant knows the reader is behind', () => {
		const url = artifactUrl('uuid-1', 'My Plan', 2)
		expect(describePreview([{ id: 'a', url, loc: url }], 'a')).toContain(
			'artifact "My Plan" (pinned to v2)'
		)
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

	it('fires on re-opening whatever the panel already displays, for any tab kind', () => {
		const o = owner()
		o.open(rawAppTarget)
		expect(o.focusPulse.nonce).toBe(0)
		o.open(rawAppTarget)
		expect(o.focusPulse).toEqual({ id: o.activeId, nonce: 1 })
		// A second tab taking over is its own visible change.
		o.open(pageTarget)
		expect(o.focusPulse.nonce).toBe(1)
		o.open(pageTarget)
		expect(o.focusPulse).toEqual({ id: o.activeId, nonce: 2 })
		// Re-pointing the displayed tab elsewhere changes what is on screen.
		o.navigate(scriptTarget)
		expect(o.focusPulse.nonce).toBe(2)
		o.navigate(scriptTarget)
		expect(o.focusPulse.nonce).toBe(3)
	})

	it('still flashes a collapsed-but-fullscreen panel', () => {
		const o = owner()
		o.open(rawAppTarget)
		o.setCollapsed(true)
		// Fullscreen carries over from the previous session and overrides collapse,
		// so the tab is on screen and a re-open of it changes nothing visible.
		o.setFullscreen(true)
		o.open(rawAppTarget)
		expect(o.focusPulse.nonce).toBe(1)
	})

	it('judges a composed select+navigate as one change', () => {
		const o = owner()
		o.open(pageTarget)
		const runs = o.tabs[0].id
		o.open(rawAppTarget)
		expect(o.focusPulse.nonce).toBe(0)
		// open_page reusing a *background* page tab: the switch is the visible change.
		o.asOneChange(() => {
			o.select(runs)
			o.navigate(pageTarget)
		})
		expect(o.focusPulse.nonce).toBe(0)
		// Same sequence once that tab is already displayed: nothing changes, so flash.
		o.asOneChange(() => {
			o.select(runs)
			o.navigate(pageTarget)
		})
		expect(o.focusPulse).toEqual({ id: runs, nonce: 1 })
	})

	it('keeps the editor-stamped label when re-pointed at the same item', () => {
		const o = owner()
		o.open(scriptTarget)
		o.setEditorFriendlyLabel({ kind: 'script', path: 'u/me/foo' }, 'My script', 'u/me/staged')
		// Nothing re-stamps a tab that never changed item, so a wipe here would be
		// permanent — and would make the "nothing changed" flash a lie.
		o.navigate(scriptTarget)
		expect(o.tabs[0].friendlyLabel).toBe('My script')
		expect(o.tabs[0].friendlyPath).toBe('u/me/staged')
		o.navigate(flowTarget)
		expect(o.tabs[0].friendlyLabel).toBeUndefined()
	})

	it('focuses (and flashes) a run tab whose href carries ?workspace=', () => {
		const o = owner()
		const run: PreviewTarget = {
			type: 'page',
			href: `${base}/run/job-1?workspace=fork`,
			label: 'Run'
		}
		o.open(run)
		// What the frame reports back has the injected params stripped.
		o.observeLocation(o.activeId, `${base}/run/job-1?workspace=fork&nomenubar=true`)
		expect(o.tabs.length).toBe(1)
		expect(o.open(run).status).toBe('focused')
		expect(o.tabs.length).toBe(1)
		expect(o.focusPulse.nonce).toBe(1)
	})

	it('stays quiet when the re-open is what reveals the tab', () => {
		const o = owner()
		o.open(rawAppTarget)
		o.setCollapsed(true)
		// Un-collapsing onto the same tab is already visible.
		o.open(rawAppTarget)
		expect(o.focusPulse.nonce).toBe(0)
		// Switching back to a background tab is too.
		o.open(pageTarget)
		o.open(rawAppTarget)
		expect(o.focusPulse.nonce).toBe(0)
	})
})
