import { afterEach, describe, expect, it, vi } from 'vitest'
import { JobService, ScriptService } from '$lib/gen'
import { createPipelineAiHelpers, type PipelineDraft } from './pipelineAiHelpers'
import type { AssetGraphResponse } from './types'

// Build a helper handle over an in-memory drafts Map, mirroring how the editor
// wires it. `getFolder` returns the bare folder name (as the route/session do).
function makeHandle(
	initial: Array<[string, PipelineDraft]> = [],
	runnables: Array<{ path: string }> = []
) {
	let drafts = new Map(initial)
	let forgotten: string[] = []
	const handle = createPipelineAiHelpers({
		getFolder: () => 'x',
		getWorkspace: () => 'w',
		getResolvedGraph: () =>
			({ assets: [], runnables, edges: [], triggers: [] }) as unknown as AssetGraphResponse,
		getDrafts: () => drafts,
		setDrafts: (next) => (drafts = next),
		newDraftLocalId: () => 'id',
		onForgetPath: (p) => forgotten.push(p)
	})
	return { handle, drafts: () => drafts, forgotten: () => forgotten }
}

afterEach(() => vi.restoreAllMocks())

const draft = (over: Partial<PipelineDraft> = {}): PipelineDraft =>
	({ localId: 'l', script: { content: '' } as any, ...over }) as PipelineDraft

describe('pipeline AI direct-draft helpers', () => {
	it('removeProposedNode discards the unsaved draft at a path', async () => {
		const { handle, drafts, forgotten } = makeHandle([
			['f/x/a', draft()],
			['f/x/b', draft()]
		])
		await handle.removeProposedNode('f/x/a')
		expect(drafts().has('f/x/a')).toBe(false)
		expect(drafts().has('f/x/b')).toBe(true)
		expect(forgotten()).toContain('f/x/a')
	})

	it('removeProposedNode throws when there is no draft to discard', async () => {
		const { handle } = makeHandle()
		await expect(handle.removeProposedNode('f/x/missing')).rejects.toThrow()
	})

	it('getPipelineContext does not expose any pending/approval state', () => {
		const { handle } = makeHandle([['f/x/a', draft()]])
		const ctx = handle.getPipelineContext()
		expect(ctx).not.toHaveProperty('pendingProposals')
		expect(handle).not.toHaveProperty('acceptAll')
		expect(handle).not.toHaveProperty('rejectAll')
	})

	it('testNode on a deployed node never dispatches downstream subscribers', async () => {
		// No draft at the path → runs the deployed version, which must carry
		// `_wmill_skip_asset_dispatch` so a single-node test can't fire downstream.
		const spy = vi.spyOn(JobService, 'runScriptByPath').mockResolvedValue('job-1' as any)
		const { handle } = makeHandle()
		await handle.testNode('f/x/deployed', { foo: 1 })
		expect(spy).toHaveBeenCalledWith(
			expect.objectContaining({
				requestBody: expect.objectContaining({ _wmill_skip_asset_dispatch: true, foo: 1 })
			})
		)
	})

	it('proposeNode rejects a path outside the open folder', async () => {
		const { handle, drafts } = makeHandle()
		await expect(
			handle.proposeNode({ path: 'f/other/n', language: 'duckdb' as any, content: '' })
		).rejects.toThrow(/open folder/)
		expect(drafts().size).toBe(0)
	})

	it('proposeNode rejects content missing the pipeline annotation', async () => {
		const { handle, drafts } = makeHandle()
		await expect(
			handle.proposeNode({ path: 'f/x/new', language: 'duckdb' as any, content: 'SELECT 1' })
		).rejects.toThrow(/pipeline annotation/)
		expect(drafts().size).toBe(0)
	})

	it('proposeNode rejects a path colliding with an existing draft', async () => {
		const { handle } = makeHandle([['f/x/a', draft()]])
		await expect(
			handle.proposeNode({ path: 'f/x/a', language: 'duckdb' as any, content: '-- pipeline' })
		).rejects.toThrow(/already exists/)
	})

	it('proposeNode rejects a path colliding with an existing deployed node', async () => {
		const { handle, drafts } = makeHandle([], [{ path: 'f/x/dep' }])
		await expect(
			handle.proposeNode({ path: 'f/x/dep', language: 'duckdb' as any, content: '-- pipeline' })
		).rejects.toThrow(/already exists/)
		expect(drafts().size).toBe(0)
	})

	it('proposeNode rejects a path that is an already-deployed script when the graph has not hydrated', async () => {
		// Empty graph (session preview can race open_preview), but a deployed script
		// exists at the path — the backend probe must still catch it.
		const spy = vi.spyOn(ScriptService, 'getScriptByPath').mockResolvedValue({} as any)
		const { handle, drafts } = makeHandle()
		await expect(
			handle.proposeNode({
				path: 'f/x/deployed',
				language: 'duckdb' as any,
				content: '-- pipeline'
			})
		).rejects.toThrow(/already exists/)
		expect(spy).toHaveBeenCalled()
		expect(drafts().size).toBe(0)
	})

	it('editNode rejects a path outside the open folder', async () => {
		const { handle, drafts } = makeHandle()
		await expect(handle.editNode('f/other/foo', '-- pipeline')).rejects.toThrow(/open folder/)
		expect(drafts().size).toBe(0)
	})

	it('editNode preserves the deployed script hash/metadata and replaces only content', async () => {
		const deployed = {
			hash: 'abc123',
			path: 'f/x/node',
			summary: 'My node',
			description: 'desc',
			tag: 'custom',
			language: 'duckdb',
			content: '-- pipeline\nSELECT 1'
		}
		vi.spyOn(ScriptService, 'getScriptByPath').mockResolvedValue(deployed as any)
		const { handle, drafts } = makeHandle()
		await handle.editNode('f/x/node', '-- pipeline\nSELECT 2')
		const d = drafts().get('f/x/node')
		expect(d?.script.hash).toBe('abc123')
		expect(d?.script.summary).toBe('My node')
		expect(d?.script.description).toBe('desc')
		expect(d?.script.tag).toBe('custom')
		expect(d?.script.content).toBe('-- pipeline\nSELECT 2')
	})
})
