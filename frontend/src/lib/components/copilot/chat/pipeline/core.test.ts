import { describe, it, expect, vi } from 'vitest'

// `../shared` transitively pulls in the monaco editor (and its CSS), which the
// node test environment can't load — mirror the sibling chat tests' stub.
vi.mock('monaco-editor', () => ({ editor: {} }))

import {
	pipelineTools,
	getPipelinePromptSection,
	type PipelineAIChatHelpers,
	type PipelineContext
} from './core'
import type { ToolCallbacks } from '../shared'

function toolByName(name: string) {
	const tool = pipelineTools.find((t) => t.def.function.name === name)
	if (!tool) throw new Error(`tool ${name} not found`)
	return tool
}

function noopCallbacks(): ToolCallbacks {
	return { setToolStatus: () => {}, removeToolStatus: () => {} }
}

const sampleContext: PipelineContext = {
	folder: 'analytics',
	mode: 'edit',
	nodes: [
		{
			path: 'f/analytics/orders',
			language: 'bun',
			unsaved: true,
			writes: ['ducklake://main/orders'],
			reads: [],
			triggers: ['schedule']
		}
	],
	assets: ['ducklake://main/orders']
}

function makeHelpers(overrides: Partial<PipelineAIChatHelpers> = {}): {
	helpers: { pipeline: PipelineAIChatHelpers }
	calls: Record<string, any[]>
} {
	const calls: Record<string, any[]> = {}
	const record =
		(name: string, ret?: any) =>
		(...args: any[]) => {
			;(calls[name] ??= []).push(args)
			return ret
		}
	const pipeline: PipelineAIChatHelpers = {
		getPipelineContext: () => sampleContext,
		getNodeBody: async (path: string) => {
			calls.getNodeBody = [...(calls.getNodeBody ?? []), [path]]
			return { language: 'bun', content: 'export async function main() { return 1 }' }
		},
		proposeNode: async (input) => {
			calls.proposeNode = [...(calls.proposeNode ?? []), [input]]
			return { path: input.path }
		},
		editNode: async (path, content) => {
			calls.editNode = [...(calls.editNode ?? []), [path, content]]
		},
		removeProposedNode: record('removeProposedNode'),
		testNode: async () => 'job-123',
		...overrides
	}
	return { helpers: { pipeline }, calls }
}

describe('pipeline tools', () => {
	it('exposes the expected tool surface', () => {
		expect(pipelineTools.map((t) => t.def.function.name).sort()).toEqual([
			'build_pipeline_node',
			'edit_pipeline_node',
			'get_pipeline_graph',
			'read_pipeline_node',
			'remove_pipeline_node',
			'test_pipeline_node'
		])
	})

	it('get_pipeline_graph returns the live context as JSON', async () => {
		const { helpers } = makeHelpers()
		const out = await toolByName('get_pipeline_graph').fn({
			args: {},
			workspace: 'w',
			helpers,
			toolCallbacks: noopCallbacks(),
			toolId: 't'
		})
		expect(JSON.parse(out)).toMatchObject({ folder: 'analytics' })
	})

	it('build_pipeline_node forwards to proposeNode and does not deploy', async () => {
		const { helpers, calls } = makeHelpers()
		const out = await toolByName('build_pipeline_node').fn({
			args: {
				path: 'f/analytics/clean',
				language: 'bun',
				content: '// pipeline\nexport async function main() {}',
				output_kind: 'ducklake'
			},
			workspace: 'w',
			helpers,
			toolCallbacks: noopCallbacks(),
			toolId: 't'
		})
		expect(calls.proposeNode?.[0]?.[0]).toMatchObject({
			path: 'f/analytics/clean',
			language: 'bun',
			outputKind: 'ducklake'
		})
		expect(out).toContain('not deployed')
	})

	it('edit_pipeline_node reads then applies an exact find/replace', async () => {
		const { helpers, calls } = makeHelpers({
			getNodeBody: async () => ({ language: 'bun', content: 'const x = 1\nconst y = 2\n' })
		})
		await toolByName('edit_pipeline_node').fn({
			args: { path: 'f/analytics/orders', old_string: 'const x = 1', new_string: 'const x = 42' },
			workspace: 'w',
			helpers,
			toolCallbacks: noopCallbacks(),
			toolId: 't'
		})
		expect(calls.editNode?.[0]?.[1]).toContain('const x = 42')
	})

	it('edit_pipeline_node surfaces a clear error when old_string is absent', async () => {
		const { helpers } = makeHelpers({
			getNodeBody: async () => ({ language: 'bun', content: 'const x = 1\n' })
		})
		await expect(
			toolByName('edit_pipeline_node').fn({
				args: { path: 'f/analytics/orders', old_string: 'NOT THERE', new_string: 'x' },
				workspace: 'w',
				helpers,
				toolCallbacks: noopCallbacks(),
				toolId: 't'
			})
		).rejects.toThrow(/was not found/)
	})

	it('mutation tools fail clearly when no pipeline editor is registered', async () => {
		await expect(
			toolByName('build_pipeline_node').fn({
				args: { path: 'f/a/b', language: 'bun', content: 'x' },
				workspace: 'w',
				helpers: {},
				toolCallbacks: noopCallbacks(),
				toolId: 't'
			})
		).rejects.toThrow(/No pipeline editor is open/)
	})

	it('test_pipeline_node requires confirmation', () => {
		expect(toolByName('test_pipeline_node').requiresConfirmation).toBe(true)
	})
})

describe('getPipelinePromptSection', () => {
	it('names the active folder and the direct-draft workflow', () => {
		const section = getPipelinePromptSection(sampleContext)
		expect(section).toContain('/pipeline/analytics')
		expect(section).toContain('build_pipeline_node')
		expect(section).toContain('directly as unsaved drafts')
	})
})
