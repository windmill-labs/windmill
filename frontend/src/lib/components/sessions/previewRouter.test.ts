import { describe, it, expect } from 'vitest'
import { parsePreviewItemRoute, previewTabLabel, resolvePreviewTab } from './previewRouter'
import type { SessionTarget } from './sessionState.svelte'

describe('parsePreviewItemRoute', () => {
	it('maps edit/get routes to item kinds', () => {
		expect(parsePreviewItemRoute('/scripts/edit/f/foo/bar')).toEqual({
			kind: 'script',
			raw_app: false,
			itemPath: 'f/foo/bar'
		})
		expect(parsePreviewItemRoute('/flows/get/u/admin/baz')).toEqual({
			kind: 'flow',
			raw_app: false,
			itemPath: 'u/admin/baz'
		})
		expect(parsePreviewItemRoute('/apps_raw/edit/f/a/b')).toEqual({
			kind: 'app',
			raw_app: true,
			itemPath: 'f/a/b'
		})
		expect(parsePreviewItemRoute('/apps/edit/f/a/b')).toEqual({
			kind: 'app',
			raw_app: false,
			itemPath: 'f/a/b'
		})
	})

	it('returns null for non-item pages', () => {
		expect(parsePreviewItemRoute('/')).toBeNull()
		expect(parsePreviewItemRoute('/runs')).toBeNull()
		expect(parsePreviewItemRoute('/workspace_settings')).toBeNull()
	})
})

describe('previewTabLabel', () => {
	it('labels a new raw app by its pending friendly path, not the draft uuid', () => {
		const rawApp = { path: 'u/admin/draft_abc123', draft_path: 'u/admin/my_pretty_app' }
		expect(previewTabLabel('/apps_raw/edit/u/admin/draft_abc123', rawApp)).toBe('my_pretty_app')
	})

	it('falls back to the uuid leaf when no friendly draft_path is pending', () => {
		const rawApp = { path: 'u/admin/draft_abc123' }
		expect(previewTabLabel('/apps_raw/edit/u/admin/draft_abc123', rawApp)).toBe('draft_abc123')
	})

	it('keeps the real leaf for a raw app already at a named (non-draft) path', () => {
		const rawApp = { path: 'u/admin/my_app', draft_path: 'u/admin/renamed' }
		expect(previewTabLabel('/apps_raw/edit/u/admin/my_app', rawApp)).toBe('my_app')
	})

	it('ignores a draft_path that belongs to a different raw app than the tab shows', () => {
		const rawApp = { path: 'u/admin/draft_other', draft_path: 'u/admin/friendly' }
		expect(previewTabLabel('/apps_raw/edit/u/admin/draft_abc123', rawApp)).toBe('draft_abc123')
	})

	it('does not touch non-raw-app tabs', () => {
		const rawApp = { path: 'u/admin/draft_abc123', draft_path: 'u/admin/friendly' }
		expect(previewTabLabel('/scripts/edit/f/foo/bar', rawApp)).toBe('bar')
		expect(previewTabLabel('/runs', rawApp)).toBe('Runs')
	})

	it('falls back to the plain location label when no raw app is loaded', () => {
		expect(previewTabLabel('/apps_raw/edit/u/admin/draft_abc123', undefined)).toBe('draft_abc123')
	})
})

describe('resolvePreviewTab', () => {
	const scriptTarget: SessionTarget = { kind: 'script', path: 'f/foo/bar' }

	it('routes a static page to the iframe fallback', () => {
		expect(resolvePreviewTab('/runs', scriptTarget)).toEqual({ kind: 'iframe' })
	})

	it('routes the matching target item to a live editor', () => {
		expect(resolvePreviewTab('/scripts/edit/f/foo/bar', scriptTarget)).toEqual({
			kind: 'editor',
			editorKind: 'script',
			path: 'f/foo/bar'
		})
	})

	it('routes a same-kind but different item to the iframe (one editor per session)', () => {
		expect(resolvePreviewTab('/scripts/edit/f/other/script', scriptTarget)).toEqual({
			kind: 'iframe'
		})
	})

	it('routes a different-kind item to the iframe even when it matches no target', () => {
		expect(resolvePreviewTab('/flows/edit/f/foo/bar', scriptTarget)).toEqual({ kind: 'iframe' })
	})

	it('maps a raw-app target to the raw_app editor kind', () => {
		const target: SessionTarget = { kind: 'raw_app', path: 'f/a/b' }
		expect(resolvePreviewTab('/apps_raw/edit/f/a/b', target)).toEqual({
			kind: 'editor',
			editorKind: 'raw_app',
			path: 'f/a/b'
		})
	})

	it('never routes a regular drag-and-drop app to an editor (no wrapper exists)', () => {
		const target = { kind: 'raw_app', path: 'f/a/b' } as SessionTarget
		expect(resolvePreviewTab('/apps/edit/f/a/b', target)).toEqual({ kind: 'iframe' })
	})

	it('falls back to the iframe when the session has no target', () => {
		expect(resolvePreviewTab('/scripts/edit/f/foo/bar', undefined)).toEqual({ kind: 'iframe' })
	})
})
