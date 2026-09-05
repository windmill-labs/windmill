import { describe, it, expect } from 'vitest'
import {
	toolReloadEffect,
	tabsToReload,
	strongerEntityEffect,
	entityEffectForTab,
	effectForWrite,
	effectForDiscard
} from './previewReload'
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

	// A hosted entity editor holds the draft cell a write seeds, so a write must
	// leave it alone — that is what makes the chat's edit show up live. The tools
	// that drop the cell have to reach it, and a delete leaves it with no item.
	it('asks a hosted entity editor to hold, refresh, or close, per tool', () => {
		expect(toolReloadEffect('write_resource', {}).entity).toBe('none')
		expect(toolReloadEffect('write_schedule', { path: 'u/me/s' }).entity).toBe('none')
		expect(toolReloadEffect('write_trigger', { kind: 'kafka' }).entity).toBe('none')
		expect(toolReloadEffect('deploy_workspace_item', { type: 'resource' }).entity).toBe('refresh')
		expect(toolReloadEffect('discard_local_draft', { type: 'variable' }).entity).toBe('refresh')
		expect(toolReloadEffect('rebase_draft', { type: 'schedule' }).entity).toBe('refresh')
		expect(toolReloadEffect('delete_workspace_item', { type: 'resource' }).entity).toBe('close')
	})

	it('carries the mutated item path, so an editor on another item is left alone', () => {
		expect(
			toolReloadEffect('delete_workspace_item', { type: 'resource', path: 'u/me/a' }).path
		).toBe('u/me/a')
		// No path in the args: nothing to scope by, so it reaches every editor on the page.
		expect(toolReloadEffect('deploy_workspace_item', { type: 'resource' }).path).toBeUndefined()
	})

	it('takes the strongest effect across a debounced round', () => {
		expect(strongerEntityEffect('none', 'refresh')).toBe('refresh')
		expect(strongerEntityEffect('refresh', 'close')).toBe('close')
		expect(strongerEntityEffect('close', 'refresh')).toBe('close')
		expect(strongerEntityEffect('none', 'none')).toBe('none')
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

describe('effectForWrite', () => {
	// A write is normally invisible to this layer — the editor holds the cell it
	// seeds. The exception is an editor whose first load is still in flight: it
	// holds no cell yet, `seed` no-ops, and only a re-read reconciles it.
	it('asks for a refresh only when the seed found no live editor', () => {
		expect(effectForWrite('none', true)).toBe('none')
		expect(effectForWrite('none', false)).toBe('refresh')
	})

	it('never weakens what the tool already asked for', () => {
		expect(effectForWrite('close', false)).toBe('close')
		expect(effectForWrite('refresh', true)).toBe('refresh')
	})
})

describe('effectForDiscard', () => {
	// Discarding reverts an item to what is deployed — unless nothing is, in which
	// case the draft WAS the item and the editor is left showing something that no
	// longer exists, remounting into a load that cannot resolve.
	it('closes a hosted editor whose item the discard removed outright', () => {
		expect(effectForDiscard('refresh', false)).toBe('close')
	})

	it('keeps a deployed item on screen, reverted rather than closed', () => {
		expect(effectForDiscard('refresh', true)).toBe('refresh')
	})

	it('leaves the other effects alone', () => {
		expect(effectForDiscard('none', false)).toBe('none')
		expect(effectForDiscard('close', true)).toBe('close')
	})
})

describe('entityEffectForTab', () => {
	const del = (path: string, workspace = 'ws1') => ({
		pages: ['/resources'],
		effect: 'close' as const,
		path,
		workspace
	})
	const tab = { listPage: '/resources', path: 'u/me/a', workspace: 'ws1' }

	it('applies a mutation to the editor on that item', () => {
		expect(entityEffectForTab([del('u/me/a')], tab)).toBe('close')
	})

	// The bug this guards: deleting one resource must not shut every open resource
	// editor, nor the same path in a session acting on another workspace.
	it('leaves editors on another item, page, or workspace alone', () => {
		expect(entityEffectForTab([del('u/me/b')], tab)).toBe('none')
		expect(entityEffectForTab([del('u/me/a', 'ws2')], tab)).toBe('none')
		expect(entityEffectForTab([{ ...del('u/me/a'), pages: ['/variables'] }], tab)).toBe('none')
	})

	it('reaches every editor on the page when the tool named no item', () => {
		expect(entityEffectForTab([{ ...del('u/me/a'), path: undefined }], tab)).toBe('close')
	})

	it('takes the strongest of the mutations that reach it, not of the whole round', () => {
		const refreshOther = {
			pages: ['/resources'],
			effect: 'refresh' as const,
			path: 'u/me/b',
			workspace: 'ws1'
		}
		const refreshMine = { ...refreshOther, path: 'u/me/a' }
		expect(entityEffectForTab([refreshMine, del('u/me/b')], tab)).toBe('refresh')
		expect(entityEffectForTab([refreshMine, del('u/me/a')], tab)).toBe('close')
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
