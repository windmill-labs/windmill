import { describe, it, expect } from 'vitest'
import {
	artifactUrl,
	draftFriendlyLeaf,
	matchReusablePage,
	parseArtifactRoute,
	parsePreviewItemRoute,
	previewLocationLabel,
	resolvePreviewTab
} from './previewRouter'

describe('matchReusablePage', () => {
	it('matches curated pages and the compare page, ignoring query params', () => {
		expect(matchReusablePage('/runs?path=f/a/b')?.path).toBe('/runs')
		expect(matchReusablePage('/forks/compare?workspace_id=ws&items=script:f/a/b')?.path).toBe(
			'/forks/compare'
		)
		expect(previewLocationLabel('/forks/compare?workspace_id=ws')).toBe('Compare & Deploy')
	})

	it('does not match trigger pages (they dedupe on exact URL)', () => {
		expect(matchReusablePage('/kafka_triggers')).toBeUndefined()
	})
})

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

describe('draftFriendlyLeaf', () => {
	it('returns the friendly leaf for a new item parked at a draft uuid', () => {
		expect(draftFriendlyLeaf('u/admin/draft_abc123', 'u/admin/valuable_script')).toBe(
			'valuable_script'
		)
		expect(draftFriendlyLeaf('u/admin/draft_abc123', 'u/admin/my_flow')).toBe('my_flow')
	})

	it('returns undefined when no friendly path is available', () => {
		expect(draftFriendlyLeaf('u/admin/draft_abc123', undefined)).toBeUndefined()
	})

	it('returns undefined when the friendly path is itself a draft placeholder', () => {
		expect(draftFriendlyLeaf('u/admin/draft_abc123', 'u/admin/draft_xyz')).toBeUndefined()
	})

	it('returns undefined for an item already at a named (non-draft) storage path', () => {
		expect(draftFriendlyLeaf('u/admin/my_app', 'u/admin/renamed')).toBeUndefined()
	})
})

describe('resolvePreviewTab', () => {
	it('routes a static page to the iframe fallback', () => {
		expect(resolvePreviewTab('/runs')).toEqual({ kind: 'iframe' })
	})

	it('routes any script item to a live editor', () => {
		expect(resolvePreviewTab('/scripts/edit/f/foo/bar')).toEqual({
			kind: 'editor',
			editorKind: 'script',
			path: 'f/foo/bar'
		})
	})

	it('routes any flow item to a live editor', () => {
		expect(resolvePreviewTab('/flows/edit/f/foo/bar')).toEqual({
			kind: 'editor',
			editorKind: 'flow',
			path: 'f/foo/bar'
		})
	})

	it('maps a raw app to the raw_app editor kind', () => {
		expect(resolvePreviewTab('/apps_raw/edit/f/a/b')).toEqual({
			kind: 'editor',
			editorKind: 'raw_app',
			path: 'f/a/b'
		})
	})

	it('never routes a regular drag-and-drop app to an editor (no wrapper exists)', () => {
		expect(resolvePreviewTab('/apps/edit/f/a/b')).toEqual({ kind: 'iframe' })
	})

	it('routes a pipeline folder to the pipeline editor kind', () => {
		expect(resolvePreviewTab('/pipeline/my_folder')).toEqual({
			kind: 'editor',
			editorKind: 'pipeline',
			path: 'my_folder'
		})
	})

	it('routes the bare pipeline list page to the iframe fallback', () => {
		expect(resolvePreviewTab('/pipeline')).toEqual({ kind: 'iframe' })
	})

	it('routes an artifact url to the artifact slot by id (ignoring the name hash)', () => {
		expect(resolvePreviewTab('artifact:abc%20123#My%20Doc')).toEqual({
			kind: 'artifact',
			id: 'abc 123'
		})
	})
})

describe('artifact route', () => {
	it('round-trips id and name through artifactUrl → parseArtifactRoute, including special chars', () => {
		for (const [id, name] of [
			['abc', 'Onboarding plan'],
			['id-with-dash', 'weird # % / name'],
			['x', 'artifact:not-an-id#nope'],
			['y', '']
		] as const) {
			expect(parseArtifactRoute(artifactUrl(id, name))).toEqual({ id, name })
		}
	})

	it('parses a hash-less artifact url to an empty name', () => {
		expect(parseArtifactRoute('artifact:abc')).toEqual({ id: 'abc', name: '' })
	})

	it('returns null for non-artifact urls', () => {
		expect(parseArtifactRoute('/scripts/edit/f/foo/bar')).toBeNull()
		expect(parseArtifactRoute('/runs')).toBeNull()
		expect(parseArtifactRoute('artifactx:abc')).toBeNull()
	})

	it('labels an artifact tab by its name, falling back to "Artifact" when unnamed', () => {
		expect(previewLocationLabel(artifactUrl('abc', 'My Doc'))).toBe('My Doc')
		expect(previewLocationLabel('artifact:abc')).toBe('Artifact')
	})
})
