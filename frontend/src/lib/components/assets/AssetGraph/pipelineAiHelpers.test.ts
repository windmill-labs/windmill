import { describe, expect, it } from 'vitest'
import { createPipelineAiHelpers, type PipelineDraft } from './pipelineAiHelpers'
import type { AssetGraphResponse } from './types'

// Build a helper handle over an in-memory drafts Map, mirroring how the editor
// wires it. Only accept/reject are exercised here (they don't touch the network
// helpers), so the graph/workspace accessors are stubbed.
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

describe('pipeline AI accept/reject', () => {
	it('acceptAll clears the aiPending flag, keeping the drafts', () => {
		const { handle, drafts } = makeHandle([['f/x/a', draft({ aiPending: true })]])
		expect(handle.hasPending()).toBe(true)
		handle.acceptAll()
		expect(drafts().has('f/x/a')).toBe(true)
		expect(drafts().get('f/x/a')?.aiPending).toBe(false)
		expect(handle.hasPending()).toBe(false)
	})

	it('rejectAll discards a rehydrated proposal that has no in-memory snapshot', () => {
		// Simulates a post-reload state: the proposal was restored from the persisted
		// draft, so the snapshot map (rebuilt fresh each mount) is empty.
		const { handle, drafts, forgotten } = makeHandle([['f/x/a', draft({ aiPending: true })]])
		handle.rejectAll()
		expect(drafts().has('f/x/a')).toBe(false)
		expect(forgotten()).toContain('f/x/a')
		expect(handle.hasPending()).toBe(false)
	})

	it('rejectAll leaves non-AI drafts untouched', () => {
		const { handle, drafts } = makeHandle([
			['f/x/ai', draft({ aiPending: true })],
			['f/x/manual', draft()]
		])
		handle.rejectAll()
		expect(drafts().has('f/x/ai')).toBe(false)
		expect(drafts().has('f/x/manual')).toBe(true)
	})
})
