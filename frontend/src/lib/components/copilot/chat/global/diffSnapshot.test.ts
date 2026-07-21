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
	ResourceService: { getResource: vi.fn(), getResourceType: vi.fn() },
	FlowService: { getFlowByPath: vi.fn() }
}))
vi.mock('./userDraftAdapter', () => ({
	itemTypeForKind: (kind: string) =>
		kind === 'script'
			? { type: 'script' }
			: kind === 'raw_app' || kind === 'app'
				? { type: 'app' }
				: kind === 'trigger_http'
					? { type: 'trigger', triggerKind: 'http' }
					: undefined
}))

import { getDraftItems, getWorkspaceDraftsVersion } from '$lib/workspaceDrafts.svelte'
import { getDraftDiffValues } from '$lib/utils_draft_deploy'
import { getItemValue } from '$lib/utils_workspace_deploy'
import { fetchWorkspaceComparison } from '$lib/workspaceComparison'
import { FlowService, ResourceService, ScriptService, VariableService } from '$lib/gen'
import {
	expireWorkspaceDiffList,
	getForkDiffIndex,
	getForkParentWorkspaceId,
	getWorkspaceDiffIndex,
	invalidateWorkspaceDiffCache,
	markWorkspaceDiffEntryStale,
	readForkDiffEntries,
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

	it('refetches a stale-marked entry immediately, despite every reuse window', async () => {
		vi.mocked(getDraftItems).mockResolvedValue([row()] as any)
		mockDiffValues({ content: 'a' }, { content: 'b' })
		await readWorkspaceDiffEntry(WS, 'script', 'f/a/b')
		expect(getDraftDiffValues).toHaveBeenCalledTimes(1)

		// Within both the list throttle and the read-reuse window the patch is
		// served from cache...
		await readWorkspaceDiffEntry(WS, 'script', 'f/a/b')
		expect(getDraftDiffValues).toHaveBeenCalledTimes(1)

		// ...but a landed save (the syncer hook) forces content + listing fresh.
		markWorkspaceDiffEntryStale(WS, 'script', 'f/a/b')
		mockDiffValues({ content: 'a' }, { content: 'c' })
		const entry = await readWorkspaceDiffEntry(WS, 'script', 'f/a/b')
		expect(getDraftDiffValues).toHaveBeenCalledTimes(2)
		expect(entry?.patch).toContain('+content: c')
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

	it('masks draft-mode variable values but still marks a value change', async () => {
		vi.mocked(getDraftItems).mockResolvedValue([
			row({ kind: 'variable', path: 'f/a/token' })
		] as any)
		vi.mocked(getDraftDiffValues).mockResolvedValue({
			deployed: { value: 'old-plaintext', is_secret: false, description: 'd' },
			draft: { value: 'new-plaintext', is_secret: false, description: 'd' },
			hasDraft: true,
			noDeployed: false
		} as any)
		const entry = await readWorkspaceDiffEntry(WS, 'variable', 'f/a/token')
		expect(entry?.status).toBe('modified')
		expect(entry?.valueMasked).toBe(true)
		// No plaintext on either side — the patch only marks that it changed.
		expect(entry?.patch).not.toContain('plaintext')
		expect(entry?.patch).toContain('(changed)')
	})

	it('flags a secret-only draft as uncomparable instead of claiming unchanged', async () => {
		vi.mocked(getDraftItems).mockResolvedValue([
			row({ kind: 'variable', path: 'f/a/secret' })
		] as any)
		// The shared canonicalizer already masked both sides to the same
		// sentinel — equality between them proves nothing.
		vi.mocked(getDraftDiffValues).mockResolvedValue({
			deployed: { value: '<secret>', is_secret: true, description: 'd' },
			draft: { value: '<secret>', is_secret: true, description: 'd' },
			hasDraft: true,
			noDeployed: false
		} as any)
		const entry = await readWorkspaceDiffEntry(WS, 'variable', 'f/a/secret')
		expect(entry?.status).toBe('unchanged')
		expect(entry?.valueUncomparable).toBe(true)
	})

	it('materializes classic-app drafts under the chat app type', async () => {
		vi.mocked(getDraftItems).mockResolvedValue([row({ kind: 'app', path: 'f/a/classic' })] as any)
		mockDiffValues({ summary: 'v1' }, { summary: 'v2' })
		const index = await getWorkspaceDiffIndex(WS)
		expect(index.entries[0].type).toBe('app')
		expect(index.entries[0].status).toBe('modified')
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

	it('reports empty-file additions and deletions as changes despite the empty patch', async () => {
		vi.mocked(getDraftItems).mockResolvedValue([
			row({ kind: 'raw_app', path: 'f/dash/main' })
		] as any)
		vi.mocked(getDraftDiffValues).mockResolvedValue({
			deployed: { summary: 'Dash', files: { 'gone.css': '', 'kept.ts': 'same\n' }, runnables: {} },
			draft: { summary: 'Dash', files: { 'added.ts': '', 'kept.ts': 'same\n' }, runnables: {} },
			hasDraft: true,
			noDeployed: false
		} as any)
		const entry = await readWorkspaceDiffEntry(WS, 'raw_app', 'f/dash/main')
		expect(entry?.status).toBe('modified')
		expect(entry?.files?.['added.ts']).toEqual({ status: 'added', patch: '', lineCount: 0 })
		expect(entry?.files?.['gone.css']).toEqual({ status: 'deleted', patch: '', lineCount: 0 })
		expect(entry?.files?.['kept.ts']).toBeUndefined()
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

	// (resolveWorkspaceDiffTarget)
	it('resolves a renamed draft to its owning row across kinds', async () => {
		vi.mocked(getDraftItems).mockResolvedValue([
			row({ kind: 'app', path: 'f/old/name', draft_path: 'f/new/name' })
		] as any)
		const { resolveWorkspaceDiffTarget } = await import('./diffSnapshot')
		const target = await resolveWorkspaceDiffTarget(WS, ['raw_app', 'app'], 'f/new/name')
		expect(target).toEqual({ kind: 'app', storagePath: 'f/old/name' })
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


async function readForkDiffEntryOne(workspace: string, parent: string, kinds: string[], path: string) {
	const entries = await readForkDiffEntries(workspace, parent, kinds, path)
	return entries[0]
}

const FORK = 'fork-ws'
const PARENT = 'parent-ws'

function comparisonDiff(overrides: Partial<Record<string, unknown>> = {}) {
	return {
		kind: 'schedule',
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

	it('never decrypts variables and masks every value, secret or not', async () => {
		mockComparison([comparisonDiff({ kind: 'variable', path: 'f/a/plain' })])
		vi.mocked(VariableService.getVariable).mockImplementation(async ({ workspace }: any) => ({
			path: 'f/a/plain',
			is_secret: false,
			value: `plaintext-${workspace}`,
			description: workspace === FORK ? 'fork side' : 'parent side'
		})) as any
		const entry = await readForkDiffEntryOne(FORK, PARENT, ['variable'], 'f/a/plain')
		expect(
			vi.mocked(VariableService.getVariable).mock.calls.every(([a]: any) => !a.decryptSecret)
		).toBe(true)
		// NON-secret values are masked too — the chat never sees variable values.
		expect(entry?.patch).not.toContain('plaintext-')
		// Description change still shows; the value itself never differs here.
		expect(entry?.patch).toContain('fork side')
		expect(entry?.valueMasked).toBe(true)
	})

	it('sees flow schema-only changes the shared projection drops', async () => {
		mockComparison([comparisonDiff({ kind: 'flow', path: 'f/a/fl' })])
		vi.mocked(FlowService.getFlowByPath).mockImplementation(async ({ workspace }: any) => ({
			summary: 'same',
			description: 'same',
			schema: { properties: workspace === FORK ? { a: {} } : { a: {}, b: {} } },
			value: { modules: [{ id: 'x', value: { type: 'script', hash: `h-${workspace}` } }] }
		})) as any
		const entry = await readForkDiffEntryOne(FORK, PARENT, ['flow'], 'f/a/fl')
		expect(entry?.status).toBe('modified')
		expect(entry?.patch).toContain('schema')
		// Inline-script hashes are per-workspace noise, never a diff line.
		expect(entry?.patch).not.toContain('h-fork-ws')
	})

	it('sees script description-only changes the shared projection drops', async () => {
		mockComparison([comparisonDiff({ kind: 'script', path: 'f/a/s' })])
		vi.mocked(ScriptService.getScriptByPath).mockImplementation(async ({ workspace }: any) => ({
			content: 'same code',
			summary: 'same',
			description: workspace === FORK ? 'fork docs' : 'parent docs',
			language: 'bun'
		})) as any
		const entry = await readForkDiffEntryOne(FORK, PARENT, ['script'], 'f/a/s')
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
		const entry = await readForkDiffEntryOne(FORK, PARENT, ['app', 'raw_app'], 'f/dash/main')
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

	it('reads fork-only kinds by path alone; a wildcard returns every matching kind', async () => {
		mockComparison([
			comparisonDiff({ kind: 'folder', path: 'f/team' }),
			comparisonDiff({ kind: 'resource_type', path: 'shared_name' }),
			comparisonDiff({ kind: 'folder', path: 'shared_name' })
		])
		vi.mocked(getItemValue).mockImplementation(async (_k, path, ws) => ({
			name: path,
			summary: `${ws}`
		}))
		vi.mocked(ResourceService.getResourceType).mockImplementation(async ({ workspace }: any) => ({
			schema: {},
			description: `${workspace}`
		})) as any
		const folder = await readForkDiffEntryOne(FORK, PARENT, [], 'f/team')
		expect(folder?.kind).toBe('folder')
		expect(folder?.status).toBe('modified')

		const both = await readForkDiffEntries(FORK, PARENT, [], 'shared_name')
		expect(both.map((e) => e.kind).sort()).toEqual(['folder', 'resource_type'])
		expect(both.every((e) => e.status === 'modified')).toBe(true)
	})

	it('flags entries that also carry a local draft', async () => {
		mockComparison([comparisonDiff(), comparisonDiff({ path: 'f/a/other' })])
		vi.mocked(getDraftItems).mockResolvedValue([
			row({ kind: 'trigger_schedule', path: 'f/a/b' })
		] as any)
		vi.mocked(getItemValue).mockResolvedValue({ content: 'x' })
		const index = await getForkDiffIndex(FORK, PARENT)
		const byPath = Object.fromEntries(index.entries.map((e) => [e.path, e]))
		expect(byPath['f/a/b'].hasLocalDraft).toBe(true)
		expect(byPath['f/a/other'].hasLocalDraft).toBe(false)
	})
})
