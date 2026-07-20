import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('$lib/workspaceDrafts.svelte', () => ({
	getDraftItems: vi.fn(),
	getWorkspaceDraftsVersion: vi.fn(() => 0)
}))
vi.mock('$lib/utils_draft_deploy', () => ({
	getDraftDiffValues: vi.fn()
}))
vi.mock('$lib/utils_workspace_deploy', () => ({
	getItemValue: vi.fn()
}))
vi.mock('$lib/workspaceComparison', () => ({
	fetchWorkspaceComparison: vi.fn()
}))
vi.mock('$lib/stores', async () => {
	const { readable } = await import('svelte/store')
	return {
		userWorkspaces: readable([{ id: 'fork-ws', parent_workspace_id: 'parent-ws' }])
	}
})
vi.mock('$lib/gen', () => ({
	VariableService: { getVariable: vi.fn() },
	ScriptService: { getScriptByPath: vi.fn() },
	ResourceService: { getResource: vi.fn() }
}))
vi.mock('./userDraftAdapter', () => ({
	itemTypeForKind: (kind: string) =>
		kind === 'script'
			? { type: 'script' }
			: kind === 'raw_app'
				? { type: 'app' }
				: kind === 'trigger_http'
					? { type: 'trigger', triggerKind: 'http' }
					: undefined
}))

import { getDraftItems, getWorkspaceDraftsVersion } from '$lib/workspaceDrafts.svelte'
import { getDraftDiffValues } from '$lib/utils_draft_deploy'
import { getItemValue } from '$lib/utils_workspace_deploy'
import { fetchWorkspaceComparison } from '$lib/workspaceComparison'
import { ScriptService, VariableService } from '$lib/gen'
import {
	expireWorkspaceDiffList,
	getForkDiffIndex,
	getForkParentWorkspaceId,
	getWorkspaceDiffIndex,
	invalidateWorkspaceDiffCache,
	readForkDiffEntry,
	readWorkspaceDiffEntry
} from './diffSnapshot'

const WS = 'test-ws'

function row(overrides: Partial<Record<string, unknown>> = {}) {
	return {
		kind: 'script',
		path: 'f/a/b',
		draft_only: false,
		legacy_draft: false,
		raw_app: false,
		can_write: true,
		mine: true,
		created_at: '2026-07-20T00:00:00Z',
		...overrides
	}
}

function mockDiffValues(deployed: unknown, draft: unknown, noDeployed = false) {
	vi.mocked(getDraftDiffValues).mockResolvedValue({
		deployed,
		draft,
		hasDraft: true,
		noDeployed
	} as any)
}

beforeEach(() => {
	vi.useFakeTimers()
	invalidateWorkspaceDiffCache()
	vi.mocked(getDraftItems).mockReset()
	vi.mocked(getDraftDiffValues).mockReset()
	vi.mocked(getWorkspaceDraftsVersion).mockReset().mockReturnValue(0)
})

afterEach(() => {
	vi.useRealTimers()
})

describe('getWorkspaceDiffIndex', () => {
	it('materializes patches once and reuses them while rows are unchanged', async () => {
		vi.mocked(getDraftItems).mockResolvedValue([row()] as any)
		mockDiffValues({ content: 'a' }, { content: 'b' })

		const first = await getWorkspaceDiffIndex(WS)
		expect(first.entries).toHaveLength(1)
		expect(first.entries[0].status).toBe('modified')
		expect(first.entries[0].patch).toContain('-content: a')
		expect(getDraftDiffValues).toHaveBeenCalledTimes(1)

		// Second access after the list-reuse window: row unchanged → no refetch.
		vi.advanceTimersByTime(10_000)
		const second = await getWorkspaceDiffIndex(WS)
		expect(second.entries[0].status).toBe('modified')
		expect(getDraftDiffValues).toHaveBeenCalledTimes(1)
	})

	it('refetches the listing after expireWorkspaceDiffList despite the reuse window', async () => {
		vi.mocked(getDraftItems).mockResolvedValue([row()] as any)
		mockDiffValues({ content: 'a' }, { content: 'b' })
		await getWorkspaceDiffIndex(WS)
		expect(getDraftItems).toHaveBeenCalledTimes(1)

		// Within the reuse window the listing is served from cache...
		await getWorkspaceDiffIndex(WS)
		expect(getDraftItems).toHaveBeenCalledTimes(1)

		// ...but an expiry (post-flush) forces a refetch immediately.
		expireWorkspaceDiffList(WS)
		await getWorkspaceDiffIndex(WS)
		expect(getDraftItems).toHaveBeenCalledTimes(2)
	})

	it('refetches an entry when its draft row created_at changes', async () => {
		vi.mocked(getDraftItems).mockResolvedValue([row()] as any)
		mockDiffValues({ content: 'a' }, { content: 'b' })
		await getWorkspaceDiffIndex(WS)
		expect(getDraftDiffValues).toHaveBeenCalledTimes(1)

		vi.advanceTimersByTime(10_000)
		vi.mocked(getDraftItems).mockResolvedValue([row({ created_at: '2026-07-20T01:00:00Z' })] as any)
		mockDiffValues({ content: 'a' }, { content: 'a' })
		const index = await getWorkspaceDiffIndex(WS)
		expect(getDraftDiffValues).toHaveBeenCalledTimes(2)
		expect(index.entries[0].status).toBe('unchanged')
	})

	it('drops cached patches when the workspace drafts version bumps', async () => {
		vi.mocked(getDraftItems).mockResolvedValue([row()] as any)
		mockDiffValues({ content: 'a' }, { content: 'b' })
		await getWorkspaceDiffIndex(WS)
		expect(getDraftDiffValues).toHaveBeenCalledTimes(1)

		vi.mocked(getWorkspaceDraftsVersion).mockReturnValue(1)
		await getWorkspaceDiffIndex(WS)
		expect(getDraftDiffValues).toHaveBeenCalledTimes(2)
	})

	it('excludes other users drafts from entries but counts them', async () => {
		vi.mocked(getDraftItems).mockResolvedValue([
			row(),
			row({ path: 'f/other/x', mine: false })
		] as any)
		mockDiffValues({ content: 'a' }, { content: 'b' })
		const index = await getWorkspaceDiffIndex(WS)
		expect(index.entries).toHaveLength(1)
		expect(index.otherUsersDraftCount).toBe(1)
	})

	it('maps statuses: never-deployed → new, unaddressable kind → not_diffable', async () => {
		vi.mocked(getDraftItems).mockResolvedValue([
			row({ path: 'f/a/new', draft_only: true }),
			row({ kind: 'data_pipeline', path: 'f/a/pipe' })
		] as any)
		mockDiffValues(undefined, { content: 'b' }, true)
		const index = await getWorkspaceDiffIndex(WS)
		const byPath = Object.fromEntries(index.entries.map((e) => [e.storagePath, e]))
		expect(byPath['f/a/new'].status).toBe('new')
		expect(byPath['f/a/pipe'].status).toBe('not_diffable')
		// Unaddressable kinds are never fetched.
		expect(getDraftDiffValues).toHaveBeenCalledTimes(1)
	})

	it('leaves entries beyond the eager cap pending', async () => {
		const rows = Array.from({ length: 55 }, (_, i) => row({ path: `f/a/s${i}` }))
		vi.mocked(getDraftItems).mockResolvedValue(rows as any)
		mockDiffValues({ content: 'a' }, { content: 'b' })
		const index = await getWorkspaceDiffIndex(WS)
		expect(index.entries).toHaveLength(55)
		expect(index.entries.filter((e) => e.status === 'pending')).toHaveLength(5)
		expect(getDraftDiffValues).toHaveBeenCalledTimes(50)
	})
})

describe('raw-app file splitting (draft mode)', () => {
	it('produces per-file patches plus a config-only patch', async () => {
		vi.mocked(getDraftItems).mockResolvedValue([
			row({ kind: 'raw_app', path: 'f/dash/main' })
		] as any)
		vi.mocked(getDraftDiffValues).mockResolvedValue({
			deployed: {
				summary: 'Dash',
				files: { 'src/App.tsx': 'a\nb\nc\n', 'old.js': 'legacy\n' },
				runnables: {}
			},
			draft: {
				summary: 'Dash v2',
				files: { 'src/App.tsx': 'a\nB\nc\n', 'new.ts': 'fresh\n' },
				runnables: {}
			},
			hasDraft: true,
			noDeployed: false
		} as any)
		const entry = await readWorkspaceDiffEntry(WS, 'raw_app', 'f/dash/main')
		expect(entry?.status).toBe('modified')
		expect(entry?.files?.['src/App.tsx'].status).toBe('modified')
		expect(entry?.files?.['src/App.tsx'].patch).toContain('-b')
		expect(entry?.files?.['new.ts'].status).toBe('added')
		expect(entry?.files?.['old.js'].status).toBe('deleted')
		// Config patch carries the summary change but no file contents.
		expect(entry?.patch).toContain('summary')
		expect(entry?.patch).not.toContain('src/App.tsx')
	})

	it('reports unchanged when neither files nor config differ', async () => {
		const value = { summary: 'Dash', files: { 'a.ts': 'same\n' }, runnables: {} }
		vi.mocked(getDraftItems).mockResolvedValue([
			row({ kind: 'raw_app', path: 'f/dash/main' })
		] as any)
		vi.mocked(getDraftDiffValues).mockResolvedValue({
			deployed: value,
			draft: structuredClone(value),
			hasDraft: true,
			noDeployed: false
		} as any)
		const entry = await readWorkspaceDiffEntry(WS, 'raw_app', 'f/dash/main')
		expect(entry?.status).toBe('unchanged')
		expect(entry?.files).toEqual({})
	})
})

describe('readWorkspaceDiffEntry', () => {
	it('resolves a draft-only item by its friendly draft_path', async () => {
		vi.mocked(getDraftItems).mockResolvedValue([
			row({ path: 'u/admin/draft_123', draft_path: 'f/nice/name', draft_only: true })
		] as any)
		mockDiffValues(undefined, { summary: 'x' }, true)
		const entry = await readWorkspaceDiffEntry(WS, 'script', 'f/nice/name')
		expect(entry?.storagePath).toBe('u/admin/draft_123')
		expect(entry?.status).toBe('new')
	})

	it('returns undefined when the user has no draft at the path', async () => {
		vi.mocked(getDraftItems).mockResolvedValue([] as any)
		const entry = await readWorkspaceDiffEntry(WS, 'script', 'f/a/b')
		expect(entry).toBeUndefined()
		expect(getDraftDiffValues).not.toHaveBeenCalled()
	})

	it('records a fetch failure as an error entry instead of throwing', async () => {
		vi.mocked(getDraftItems).mockResolvedValue([row()] as any)
		vi.mocked(getDraftDiffValues).mockRejectedValue({ status: 404 })
		const entry = await readWorkspaceDiffEntry(WS, 'script', 'f/a/b')
		expect(entry?.status).toBe('error')
		expect(entry?.errorMessage).toContain('not found')
	})
})

const FORK = 'fork-ws'
const PARENT = 'parent-ws'

function comparisonDiff(overrides: Partial<Record<string, unknown>> = {}) {
	return {
		kind: 'flow',
		path: 'f/a/b',
		ahead: 1,
		behind: 0,
		has_changes: true,
		exists_in_source: true,
		exists_in_fork: true,
		...overrides
	}
}

function mockComparison(diffs: unknown[]) {
	vi.mocked(fetchWorkspaceComparison).mockResolvedValue({
		skipped_comparison: false,
		diffs,
		summary: { total_diffs: diffs.length },
		hidden_ahead: { total: 0, by_kind: {}, items: [] },
		hidden_behind: { total: 0, by_kind: {}, items: [] }
	} as any)
}

describe('fork mode', () => {
	beforeEach(() => {
		vi.mocked(getDraftItems).mockResolvedValue([] as any)
		vi.mocked(getItemValue).mockReset()
		vi.mocked(fetchWorkspaceComparison).mockReset()
		vi.mocked(VariableService.getVariable).mockReset()
	})

	it('resolves the parent workspace from the user workspaces store', () => {
		expect(getForkParentWorkspaceId(FORK)).toBe(PARENT)
		expect(getForkParentWorkspaceId('plain-ws')).toBeUndefined()
	})

	it('diffs both deployed sides and maps one-sided items', async () => {
		mockComparison([
			comparisonDiff(),
			comparisonDiff({ path: 'f/a/new', exists_in_source: false }),
			comparisonDiff({ path: 'f/a/gone', exists_in_fork: false, ahead: 0, behind: 1 })
		])
		vi.mocked(getItemValue).mockImplementation(async (_k, path, ws) => ({
			content: `${path}@${ws}`
		}))
		const index = await getForkDiffIndex(FORK, PARENT)
		const byPath = Object.fromEntries(index.entries.map((e) => [e.path, e]))
		expect(byPath['f/a/b'].status).toBe('modified')
		expect(byPath['f/a/new'].status).toBe('only_in_fork')
		expect(byPath['f/a/new'].patch).not.toContain('parent-ws')
		expect(byPath['f/a/gone'].status).toBe('deleted_in_fork')
		// One-sided entries fetch only the existing side: 2 + 1 + 1 calls.
		expect(getItemValue).toHaveBeenCalledTimes(4)
	})

	it('reuses patches while the item ahead/behind marker is unchanged, refetches when it moves', async () => {
		mockComparison([comparisonDiff()])
		vi.mocked(getItemValue).mockResolvedValue({ content: 'x' })
		await getForkDiffIndex(FORK, PARENT)
		expect(getItemValue).toHaveBeenCalledTimes(2)

		// Past the comparison reuse window, same marker → comparison refetched
		// but the patch is kept.
		vi.advanceTimersByTime(31_000)
		await getForkDiffIndex(FORK, PARENT)
		expect(getItemValue).toHaveBeenCalledTimes(2)

		// Marker moved → content refetched.
		vi.advanceTimersByTime(31_000)
		mockComparison([comparisonDiff({ ahead: 2 })])
		await getForkDiffIndex(FORK, PARENT)
		expect(getItemValue).toHaveBeenCalledTimes(4)
	})

	it('never decrypts variables and redacts secret values', async () => {
		mockComparison([comparisonDiff({ kind: 'variable', path: 'f/a/secret' })])
		vi.mocked(VariableService.getVariable).mockImplementation(async ({ workspace }: any) => ({
			path: 'f/a/secret',
			is_secret: true,
			value: `cipher-${workspace}`,
			description: workspace === FORK ? 'fork side' : 'parent side'
		})) as any
		const entry = await readForkDiffEntry(FORK, PARENT, ['variable'], 'f/a/secret')
		expect(
			vi.mocked(VariableService.getVariable).mock.calls.every(([a]: any) => !a.decryptSecret)
		).toBe(true)
		expect(entry?.patch).not.toContain('cipher-')
		// Description change still shows; the secret value itself never differs.
		expect(entry?.patch).toContain('fork side')
		expect(entry?.secretMasked).toBe(true)
	})

	it('sees script description-only changes the shared projection drops', async () => {
		mockComparison([comparisonDiff({ kind: 'script', path: 'f/a/s' })])
		vi.mocked(ScriptService.getScriptByPath).mockImplementation(async ({ workspace }: any) => ({
			content: 'same code',
			summary: 'same',
			description: workspace === FORK ? 'fork docs' : 'parent docs',
			language: 'bun'
		})) as any
		const entry = await readForkDiffEntry(FORK, PARENT, ['script'], 'f/a/s')
		expect(entry?.status).toBe('modified')
		expect(entry?.patch).toContain('-description: parent docs')
	})

	it('splits a raw app into per-file patches with cross-workspace noise stripped', async () => {
		mockComparison([comparisonDiff({ kind: 'raw_app', path: 'f/dash/main' })])
		vi.mocked(getItemValue).mockImplementation(async (_k, _path, ws) => ({
			raw_app: true,
			summary: 'Dash',
			versions: ws === FORK ? [7] : [3],
			workspace_id: ws,
			value: {
				files:
					ws === FORK
						? { 'src/App.tsx': 'new content\n', 'src/added.ts': 'brand new\n' }
						: { 'src/App.tsx': 'old content\n', 'src/removed.ts': 'gone\n' },
				runnables: {}
			}
		}))
		const entry = await readForkDiffEntry(FORK, PARENT, ['app', 'raw_app'], 'f/dash/main')
		expect(entry?.status).toBe('modified')
		expect(entry?.files?.['src/App.tsx'].status).toBe('modified')
		expect(entry?.files?.['src/App.tsx'].patch).toContain('-old content')
		expect(entry?.files?.['src/added.ts'].status).toBe('added')
		expect(entry?.files?.['src/removed.ts'].status).toBe('deleted')
		// Version counters and workspace ids never appear as config changes.
		expect(entry?.patch).not.toContain('versions')
		expect(entry?.patch).not.toContain('workspace_id')
		expect(entry?.patch).not.toContain('parent_version')
	})

	it('flags entries that also carry a local draft', async () => {
		mockComparison([comparisonDiff(), comparisonDiff({ path: 'f/a/other' })])
		vi.mocked(getDraftItems).mockResolvedValue([row({ kind: 'flow', path: 'f/a/b' })] as any)
		vi.mocked(getItemValue).mockResolvedValue({ content: 'x' })
		const index = await getForkDiffIndex(FORK, PARENT)
		const byPath = Object.fromEntries(index.entries.map((e) => [e.path, e]))
		expect(byPath['f/a/b'].hasLocalDraft).toBe(true)
		expect(byPath['f/a/other'].hasLocalDraft).toBe(false)
	})
})
