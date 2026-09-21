import { describe, expect, it, vi } from 'vitest'

vi.mock('monaco-editor', () => ({
	editor: {},
	languages: {},
	KeyCode: {},
	Uri: { parse: (value: string) => ({ toString: () => value }) },
	MarkerSeverity: { Error: 8, Warning: 4, Info: 2, Hint: 1 }
}))
vi.mock('@codingame/monaco-vscode-standalone-typescript-language-features', () => ({
	getTypeScriptWorker: async () => async () => ({}),
	typescriptVersion: 'test'
}))
vi.mock('@codingame/monaco-vscode-languages-service-override', () => ({ default: () => ({}) }))
vi.mock('$lib/components/vscode', () => ({}))

import { getSessionContextPromptSection, type SessionPromptContext } from './core'
import { getPipelinePromptSection, type PipelineContext } from '../pipeline/core'
import { assembleGlobalSystemMessage, assembleGlobalTools } from './globalAssembly'

const sessionContext: SessionPromptContext = { workspaceId: 'ws', parentWorkspaceId: 'parent' }
const pipelineContext: PipelineContext = {
	folder: 'myfolder',
	mode: 'edit',
	nodes: [],
	assets: []
}

describe('assembleGlobalTools', () => {
	it('includes the pipeline tools only when a pipeline editor is active', () => {
		const names = (opts: Parameters<typeof assembleGlobalTools>[0]) =>
			assembleGlobalTools(opts).map((t) => t.def.function.name)
		expect(names({ pipelineContext })).toContain('get_pipeline_graph')
		expect(names({})).not.toContain('get_pipeline_graph')
	})
})

describe('assembleGlobalSystemMessage', () => {
	// The hazard this module exists to remove: every caller gets every section, so a
	// mid-session rebuild (update_user_instructions) cannot silently drop one.
	it('carries both optional sections when both contexts are given', () => {
		const content = assembleGlobalSystemMessage(undefined, {
			sessionContext,
			pipelineContext
		}).content as string
		expect(content).toContain(getSessionContextPromptSection(sessionContext))
		expect(content).toContain(getPipelinePromptSection(pipelineContext))
		// Order is part of the assembled output, so it is pinned too.
		expect(content.indexOf(getSessionContextPromptSection(sessionContext))).toBeLessThan(
			content.indexOf(getPipelinePromptSection(pipelineContext))
		)
	})

	it('omits them when the contexts are absent', () => {
		const content = assembleGlobalSystemMessage(undefined, {}).content as string
		expect(content).not.toContain(getSessionContextPromptSection(sessionContext))
		// The section's own heading, read from the code rather than retyped: a reword
		// cannot quietly make this vacuous, and it catches a stray section for any folder.
		expect(content).not.toContain(getPipelinePromptSection(pipelineContext).trim().split('\n')[0])
	})
})
