import { describe, it, expect, vi } from 'vitest'

// Mock modules that transitively import CSS/Monaco
vi.mock('monaco-editor', () => ({}))
vi.mock('@xyflow/svelte', () => ({}))
vi.mock('./renderers/nodes/AssetNode.svelte', () => ({
	assetDisplaysAsOutputInFlowGraph: () => false
}))
vi.mock('../modulesTest.svelte', () => ({}))

import { graphBuilder, topologicalSort } from './graphBuilder.svelte'
import { buildStructureTree } from './flowStructure'
import type { FlowModule } from '$lib/gen'
import type { GraphModuleState } from './model'

const script = (id: string): FlowModule => ({
	id,
	value: { type: 'rawscript', language: 'bun', content: '', input_transforms: {} }
})

describe('graphBuilder error handler markers', () => {
	it("anchors an expanded subflow's error handler to the subflow step that failed", () => {
		const modules: FlowModule[] = [
			{ id: 'sub', value: { type: 'flow', path: 'u/admin/child', input_transforms: {} } }
		]
		const ran = (parent_module: string) =>
			({ type: 'Success', args: {}, parent_module }) as GraphModuleState
		const { nodes, edges } = graphBuilder(
			buildStructureTree(modules, []),
			modules,
			{
				disableAi: true,
				insertable: false,
				flowModuleStates: { 'subflow:sub:failure': ran('bad') },
				testModuleStates: undefined,
				selectedId: undefined,
				path: undefined,
				newFlow: false,
				cache: false,
				earlyStop: false,
				editMode: false,
				isOwner: false,
				isRunning: false,
				individualStepTests: false,
				showJobStatus: true,
				flowHasChanged: false,
				chatInputEnabled: false
			},
			undefined,
			undefined,
			{} as any,
			true,
			false,
			undefined,
			undefined,
			undefined,
			{ sub: { modules: [script('bad')], failureModule: script('failure') } },
			false,
			new Set()
		)

		expect(nodes['subflow:sub:failure']).toBeDefined()
		expect(
			edges.some((e) => e.source === 'subflow:sub:bad' && e.target === 'subflow:sub:failure')
		).toBe(true)
	})
})

describe('topologicalSort', () => {
	it('skips parent ids with no node instead of throwing', () => {
		const sorted = topologicalSort([
			{ id: 'a' },
			{ id: 'b', parentIds: ['a'] },
			{ id: 'orphan', parentIds: ['deleted'] }
		])

		expect(sorted.map((n) => n.id)).toEqual(['orphan', 'b', 'a'])
	})
})
