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
	helpers: { pipelines: () => PipelineAIChatHelpers[] }
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
		getFolder: () => sampleContext.folder,
		getPipelineContext: () => sampleContext,
		getNodeBody: async (path: string) => {
			calls.getNodeBody = [...(calls.getNodeBody ?? []), [path]]
			return { language: 'bun', content: 'export async function main() { return 1 }' }
		},
		proposeNode: async (input) => {
			calls.proposeNode = [...(calls.proposeNode ?? []), [input]]
			return { path: input.path, detectedReads: [], detectedWrites: [] }
		},
		editNode: async (path, content) => {
			calls.editNode = [...(calls.editNode ?? []), [path, content]]
			return { detectedReads: [], detectedWrites: [] }
		},
		removeProposedNode: record('removeProposedNode'),
		testNode: async () => 'job-123',
		...overrides
	}
	return { helpers: { pipelines: () => [pipeline] }, calls }
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
		const modified: string[] = []
		const out = await toolByName('build_pipeline_node').fn({
			args: {
				path: 'f/analytics/clean',
				language: 'bun',
				content: '// pipeline\nexport async function main() {}',
				output_kind: 'ducklake'
			},
			workspace: 'w',
			helpers,
			toolCallbacks: {
				...noopCallbacks(),
				onItemModified: (kind, path) => modified.push(`${kind}:${path}`)
			},
			toolId: 't'
		})
		// The session tracks the folder's draft bundle, the unit it deploys.
		expect(modified).toEqual(['data_pipeline:f/analytics/data_pipeline'])
		expect(calls.proposeNode?.[0]?.[0]).toMatchObject({
			path: 'f/analytics/clean',
			language: 'bun',
			outputKind: 'ducklake'
		})
		expect(out).toContain('not deployed')
	})

	it('build_pipeline_node reports the inferred asset lineage', async () => {
		const { helpers } = makeHelpers({
			proposeNode: async (input) => ({
				path: input.path,
				detectedReads: ['s3:///raw/in.csv'],
				detectedWrites: ['ducklake://main/out']
			})
		})
		const out = await toolByName('build_pipeline_node').fn({
			args: { path: 'f/analytics/clean', language: 'duckdb', content: '-- pipeline' },
			workspace: 'w',
			helpers,
			toolCallbacks: noopCallbacks(),
			toolId: 't'
		})
		expect(out).toContain('writes ducklake://main/out')
		expect(out).toContain('reads s3:///raw/in.csv')
	})

	it('build_pipeline_node warns when no asset lineage is inferred', async () => {
		const { helpers } = makeHelpers({
			proposeNode: async (input) => ({ path: input.path, detectedReads: [], detectedWrites: [] })
		})
		const out = await toolByName('build_pipeline_node').fn({
			args: { path: 'f/analytics/clean', language: 'duckdb', content: '-- pipeline' },
			workspace: 'w',
			helpers,
			toolCallbacks: noopCallbacks(),
			toolId: 't'
		})
		expect(out).toMatch(/no storage-asset read or write was inferred/i)
		expect(out).toContain('string literal')
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

	it('remove_pipeline_node records no change: undoing a draft leaves nothing to deploy', async () => {
		const { helpers } = makeHelpers()
		const modified: string[] = []
		await toolByName('remove_pipeline_node').fn({
			args: { path: 'f/analytics/clean' },
			workspace: 'w',
			helpers,
			toolCallbacks: {
				...noopCallbacks(),
				onItemModified: (kind, path) => modified.push(`${kind}:${path}`)
			},
			toolId: 't'
		})
		expect(modified).toEqual([])
	})

	it('test_pipeline_node requires confirmation', () => {
		expect(toolByName('test_pipeline_node').requiresConfirmation).toBe(true)
	})
})

describe('pipeline tools with several editors open', () => {
	function editorFor(folder: string, built: string[]): PipelineAIChatHelpers {
		return {
			...makeHelpers().helpers.pipelines()[0],
			getFolder: () => folder,
			getPipelineContext: () => ({ ...sampleContext, folder }),
			proposeNode: async (input) => {
				built.push(`${folder}:${input.path}`)
				return { path: input.path, detectedReads: [], detectedWrites: [] }
			}
		}
	}
	const call = (name: string, args: any, editors: PipelineAIChatHelpers[]) =>
		toolByName(name).fn({
			args,
			workspace: 'w',
			helpers: { pipelines: () => editors },
			toolCallbacks: noopCallbacks(),
			toolId: 't'
		})

	it('routes a node to the editor of its folder', async () => {
		const built: string[] = []
		const editors = [editorFor('crm', built), editorFor('sales', built)]
		await call(
			'build_pipeline_node',
			{ path: 'f/sales/clean', language: 'bun', content: '// pipeline' },
			editors
		)
		expect(built).toEqual(['sales:f/sales/clean'])
		await expect(
			call(
				'build_pipeline_node',
				{ path: 'f/other/clean', language: 'bun', content: '// pipeline' },
				editors
			)
		).rejects.toThrow(/open_preview\(kind="pipeline", path="other"\)/)
	})

	it('get_pipeline_graph needs the folder only when it is ambiguous', async () => {
		const editors = [editorFor('crm', []), editorFor('sales', [])]
		await expect(call('get_pipeline_graph', {}, editors)).rejects.toThrow(/pass the folder/)
		expect(
			JSON.parse(await call('get_pipeline_graph', { folder: 'f/sales' }, editors))
		).toMatchObject({ folder: 'sales' })
		expect(JSON.parse(await call('get_pipeline_graph', {}, [editors[0]]))).toMatchObject({
			folder: 'crm'
		})
	})
})

describe('getPipelinePromptSection', () => {
	it('names the active folder and the direct-draft workflow', () => {
		const section = getPipelinePromptSection([sampleContext.folder])
		expect(section).toContain('/pipeline/analytics')
		expect(section).toContain('build_pipeline_node')
		expect(section).toContain('directly as unsaved drafts')
	})
})
