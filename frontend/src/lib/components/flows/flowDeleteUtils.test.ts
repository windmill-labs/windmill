import { describe, expect, it, vi } from 'vitest'

vi.mock('../aiProviderStorage', () => ({
	loadStoredConfig: () => undefined
}))

vi.mock('./flowInfers', () => ({
	AI_AGENT_SCHEMA: { properties: {} }
}))

import type { FlowModule } from '$lib/gen'
import type { FlowStructureNode } from '$lib/components/graph/flowStructure'
import { partitionDeleteTargets, removeToolIds } from './flowDeleteUtils'

function makeRawModule(id: string): FlowModule {
	return {
		id,
		summary: id,
		value: { type: 'rawscript', content: '', language: 'python3' } as any
	} as FlowModule
}

function makeAiAgent(id: string, tools: any[]): FlowModule {
	return {
		id,
		summary: id,
		value: {
			type: 'aiagent',
			tools,
			input_transforms: {}
		} as any
	} as FlowModule
}

function makeFlowModuleTool(module: FlowModule) {
	return {
		id: module.id,
		summary: module.summary,
		value: {
			tool_type: 'flowmodule',
			...module.value
		}
	}
}

describe('partitionDeleteTargets', () => {
	it('splits structure nodes from AI tool ids in one pass', () => {
		const tree: FlowStructureNode[] = [
			{ id: 'step_a', kind: 'leaf', branches: [] },
			{
				id: 'loop',
				kind: 'forloopflow',
				branches: [{ children: [{ id: 'nested_step', kind: 'leaf', branches: [] }] }]
			}
		]

		expect(partitionDeleteTargets(tree, ['step_a', 'tool_x', 'nested_step'])).toEqual({
			structureIds: ['step_a', 'nested_step'],
			toolIds: ['tool_x']
		})
	})
})

describe('removeToolIds', () => {
	it('returns only the ids that were actually removed', () => {
		const rootAgent = makeAiAgent('root_agent', [
			makeFlowModuleTool(makeRawModule('lookup_user')),
			makeFlowModuleTool(makeAiAgent('nested_agent', [makeFlowModuleTool(makeRawModule('create_ticket'))]))
		])
		const removed: string[] = []

		expect(
			removeToolIds([rootAgent], ['lookup_user', 'missing_tool', 'create_ticket'], (tool) => {
				removed.push(tool.id)
			})
		).toEqual(['lookup_user', 'create_ticket'])
		expect((rootAgent.value as any).tools).toHaveLength(1)
		expect((((rootAgent.value as any).tools as any[])[0].value as any).tools).toEqual([])
		expect(removed).toEqual(['lookup_user', 'create_ticket'])
	})
})
