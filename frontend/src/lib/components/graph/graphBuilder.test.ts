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

const ran = (parent_module: string) =>
	({ type: 'Success', args: {}, parent_module }) as GraphModuleState

function build(
	modules: FlowModule[],
	flowModuleStates: Record<string, GraphModuleState>,
	expandedSubflows: Record<string, any> = {}
) {
	return graphBuilder(
		buildStructureTree(modules, []),
		modules,
		{
			disableAi: true,
			insertable: false,
			flowModuleStates,
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
		script('failure'),
		undefined,
		{} as any,
		true,
		false,
		undefined,
		undefined,
		undefined,
		expandedSubflows,
		false,
		new Set()
	)
}

const hasEdge = (edges: { source: string; target: string }[], source: string, target: string) =>
	edges.some((e) => e.source === source && e.target === target)

describe('graphBuilder error handler markers', () => {
	it("anchors an expanded subflow's error handler to the subflow step that failed", () => {
		const modules: FlowModule[] = [
			{ id: 'sub', value: { type: 'flow', path: 'u/admin/child', input_transforms: {} } }
		]
		const { nodes, edges } = build(
			modules,
			{ 'subflow:sub:failure': ran('bad') },
			{ sub: { modules: [script('bad')], failureModule: script('failure') } }
		)

		expect(nodes['subflow:sub:failure']).toBeDefined()
		expect(hasEdge(edges, 'subflow:sub:bad', 'subflow:sub:failure')).toBe(true)
		expect(hasEdge(edges, 'subflow:sub:failure', 'sub-subflow-end')).toBe(true)
	})

	// The engine runs the handler in the innermost loop iteration or branch-all branch, whose
	// result is one entry of that node's array; a branch-one hands its branch's result through.
	it('leads the handler to the node that receives its output', () => {
		const modules: FlowModule[] = [
			{
				id: 'loop',
				value: {
					type: 'forloopflow',
					iterator: { type: 'javascript', expr: '[1]' },
					skip_failures: false,
					modules: [script('s')]
				}
			},
			{ id: 'bo', value: { type: 'branchone', default: [script('d')], branches: [] } }
		]
		const { edges } = build(modules, { 'failure-0-0': ran('s'), 'failure-1': ran('d') })

		expect(hasEdge(edges, 'failure-0-0', 'loop-end')).toBe(true)
		expect(hasEdge(edges, 'failure-1', 'Result')).toBe(true)
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
