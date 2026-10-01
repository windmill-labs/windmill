import { describe, expect, it, vi } from 'vitest'

const createScript = vi.fn()
vi.mock('$lib/gen', () => ({
	ScriptService: { createScript: (a: unknown) => createScript(a) },
	DraftService: {}
}))
vi.mock('$lib/infer', () => ({
	inferArgs: async () => undefined,
	inferAssets: async () => ({ status: 'ok', assets: [] })
}))
vi.mock('./pipelineTriggerDraftDeploy', () => ({ deployTriggerDraft: async () => true }))

const { deployPipelineDrafts } = await import('./pipelineDeploy.svelte')
const { PipelineEditorState } = await import('./pipelineEditorState.svelte')

const draft = (content: string) => ({
	localId: 'pe-1',
	script: { path: 'f/x/n', language: 'duckdb', content, hash: 'aaaa' } as any
})

describe('deployPipelineDrafts', () => {
	it('drops a draft once what it holds is deployed', async () => {
		const pe = new PipelineEditorState()
		pe.drafts = new Map([['f/x/n', draft('SELECT 2')]])
		createScript.mockResolvedValueOnce('bbbb')
		await deployPipelineDrafts(pe, 'ws')
		expect(pe.drafts.has('f/x/n')).toBe(false)
	})

	it('keeps edits made while the deploy was in flight, on top of the new version', async () => {
		const pe = new PipelineEditorState()
		pe.drafts = new Map([['f/x/n', draft('SELECT 2')]])
		createScript.mockImplementationOnce(async () => {
			pe.drafts = new Map([['f/x/n', draft('SELECT 3')]])
			return 'bbbb'
		})
		await deployPipelineDrafts(pe, 'ws')
		expect(pe.drafts.get('f/x/n')?.script.content).toBe('SELECT 3')
		expect(pe.drafts.get('f/x/n')?.script.hash).toBe('bbbb')
	})
})
