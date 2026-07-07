import { describe, it, expect } from 'vitest'
import { parsePreviewItemRoute, resolvePreviewTab } from './previewRouter'
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
