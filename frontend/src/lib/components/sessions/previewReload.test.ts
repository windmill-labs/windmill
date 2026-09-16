import { describe, it, expect } from 'vitest'
import { toolReloadEffect, tabsToReload, viewerTabsToReload } from './previewReload'
import type { SessionPreviewTab } from './sessionState.svelte'

describe('toolReloadEffect', () => {
	it('maps a non-item mutation to its own list page only', () => {
		expect(toolReloadEffect('write_schedule', { path: 'u/me/s' }).pages).toEqual(['/schedules'])
		expect(toolReloadEffect('write_resource', {}).pages).toEqual(['/resources'])
		expect(toolReloadEffect('write_variable', {}).pages).toEqual(['/variables'])
		expect(toolReloadEffect('create_folder', { name: 'f' }).pages).toEqual(['/folders'])
	})

	it('maps a trigger write to its kind-specific page', () => {
		expect(toolReloadEffect('write_trigger', { kind: 'kafka' }).pages).toEqual(['/kafka_triggers'])
		expect(toolReloadEffect('write_trigger', { kind: 'http' }).pages).toEqual(['/routes'])
	})

	it('maps a generic item tool to the page for its type', () => {
		expect(toolReloadEffect('deploy_workspace_item', { type: 'schedule' }).pages).toEqual([
			'/schedules'
		])
		expect(toolReloadEffect('delete_workspace_item', { type: 'resource' }).pages).toEqual([
			'/resources'
		])
		expect(
			toolReloadEffect('discard_local_draft', { type: 'trigger', trigger_kind: 'nats' }).pages
		).toEqual(['/nats_triggers'])
	})

	it('reloads no page for item-editor kinds (they self-sync via their live editor)', () => {
		for (const type of ['script', 'flow', 'app']) {
			expect(toolReloadEffect('deploy_workspace_item', { type }).pages).toEqual([])
		}
		for (const name of [
			'write_script',
			'edit_script',
			'write_flow',
			'init_app',
			'write_app_file'
		]) {
			expect(toolReloadEffect(name, { path: 'u/me/x' }).pages).toEqual([])
		}
	})

	it('reloads nothing for a purely local or unknown tool (the silent-stale guard)', () => {
		expect(toolReloadEffect('update_user_instructions', {}).pages).toEqual([])
		expect(toolReloadEffect('some_future_tool', { path: 'p' }).pages).toEqual([])
	})

	it('reloads nothing for a trigger of unknown kind rather than guessing', () => {
		expect(toolReloadEffect('write_trigger', { kind: 'not_a_kind' }).pages).toEqual([])
	})
})

describe('tabsToReload', () => {
	const scheduleTab: SessionPreviewTab = { id: 's', url: '/schedules', loc: '/schedules' }
	const resourceTab: SessionPreviewTab = { id: 'r', url: '/resources', loc: '/resources' }
	const scriptTab: SessionPreviewTab = {
		id: 'sc',
		url: '/scripts/edit/f/foo/bar',
		loc: '/scripts/edit/f/foo/bar'
	}
	const pipelineTab: SessionPreviewTab = { id: 'p', url: '/pipeline/crm', loc: '/pipeline/crm' }
	const tabs = [scheduleTab, resourceTab, scriptTab, pipelineTab]

	it('returns only the tabs whose page is in the set', () => {
		expect(tabsToReload(tabs, new Set(['/schedules']))).toEqual([scheduleTab])
	})

	it('returns list-page tabs but never item-editor or pipeline tabs', () => {
		// toolReloadEffect only ever emits list-page paths, so item/pipeline route
		// paths are never in `pages` — those tabs self-sync and stay put.
		expect(tabsToReload(tabs, new Set(['/schedules', '/resources']))).toEqual([
			scheduleTab,
			resourceTab
		])
	})

	it('is empty when no pages were touched', () => {
		expect(tabsToReload(tabs, new Set())).toEqual([])
	})

	it('matches on the observed loc (with query/hash stripped) over the seeded url', () => {
		const navigated: SessionPreviewTab = { id: 'n', url: '/runs', loc: '/schedules?workspace=w' }
		expect(tabsToReload([navigated], new Set(['/schedules']))).toEqual([navigated])
	})
})

describe('viewerTabsToReload', () => {
	const viewTab: SessionPreviewTab = {
		id: 'v',
		url: '/scripts/get/f/foo/bar',
		loc: '/scripts/get/f/foo/bar'
	}
	const editTab: SessionPreviewTab = {
		id: 'e',
		url: '/flows/edit/f/foo/baz',
		loc: '/flows/edit/f/foo/baz'
	}
	const pageTab: SessionPreviewTab = { id: 'p', url: '/schedules', loc: '/schedules' }

	it('returns the view-side tab whose item was deployed', () => {
		expect(viewerTabsToReload([viewTab, editTab, pageTab], new Set(['f/foo/bar']))).toEqual([
			viewTab
		])
	})

	// The editor holds what the user is writing; refetching the deployed version over it
	// would discard that, and it self-syncs from the store the chat already mutated.
	it('never returns the edit side, even for the item that was deployed', () => {
		expect(viewerTabsToReload([editTab], new Set(['f/foo/baz']))).toEqual([])
	})

	it('leaves a view tab on a different item alone', () => {
		expect(viewerTabsToReload([viewTab], new Set(['f/other/item']))).toEqual([])
	})

	it('is empty when no items were deployed', () => {
		expect(viewerTabsToReload([viewTab, editTab], new Set())).toEqual([])
	})
})

describe('toolReloadEffect items', () => {
	// A deploy is invisible to a View tab otherwise: it renders the deployed version over
	// the API and shares no store with the chat.
	it('names the item a deploy or delete changed, for the kinds a tab can view', () => {
		for (const type of ['script', 'flow', 'app']) {
			expect(toolReloadEffect('deploy_workspace_item', { type, path: 'u/me/x' }).items).toEqual([
				'u/me/x'
			])
			expect(toolReloadEffect('delete_workspace_item', { type, path: 'u/me/x' }).items).toEqual([
				'u/me/x'
			])
		}
	})

	it('names no item for a draft-only tool — nothing deployed changed', () => {
		expect(toolReloadEffect('write_script', { path: 'u/me/x' }).items).toEqual([])
		expect(
			toolReloadEffect('discard_local_draft', { type: 'script', path: 'u/me/x' }).items
		).toEqual([])
		expect(toolReloadEffect('rebase_draft', { type: 'flow', path: 'u/me/x' }).items).toEqual([])
	})

	it('names no item for types that have a list page instead of a view side', () => {
		expect(
			toolReloadEffect('deploy_workspace_item', { type: 'schedule', path: 'u/me/s' }).items
		).toEqual([])
	})
})
