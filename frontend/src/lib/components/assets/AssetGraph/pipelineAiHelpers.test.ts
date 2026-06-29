import { describe, expect, it } from 'vitest'
import { createPipelineAiHelpers, type PipelineDraft } from './pipelineAiHelpers'
import type { AssetGraphResponse } from './types'

// Build a helper handle over an in-memory drafts Map, mirroring how the editor
// wires it. Only the synchronous draft-mutation path (discard) is exercised here,
// so the graph/workspace accessors are stubbed.
function makeHandle(initial: Array<[string, PipelineDraft]> = []) {
	let drafts = new Map(initial)
	let forgotten: string[] = []
	const handle = createPipelineAiHelpers({
		getFolder: () => 'f/x',
		getWorkspace: () => 'w',
		getResolvedGraph: () =>
			({ assets: [], runnables: [], edges: [], triggers: [] }) as AssetGraphResponse,
		getDrafts: () => drafts,
		setDrafts: (next) => (drafts = next),
		newDraftLocalId: () => 'id',
		onForgetPath: (p) => forgotten.push(p)
	})
	return { handle, drafts: () => drafts, forgotten: () => forgotten }
}

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
})
