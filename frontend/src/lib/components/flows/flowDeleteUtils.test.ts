import { describe, expect, it, vi } from 'vitest'

vi.mock('../aiProviderStorage', () => ({
	loadStoredConfig: () => undefined
}))

vi.mock('./flowInfers', () => ({
	AI_AGENT_SCHEMA: { properties: {} }
}))

import type { FlowModule, OpenFlow } from '$lib/gen'
import type { FlowStructureNode } from '$lib/components/graph/flowStructure'
import { GroupDisplayState } from '$lib/components/graph/groupEditor.svelte'
import type { GroupedModulesProxy } from '$lib/components/graph/groupedModulesProxy.svelte'
import {
	createDeletePlan,
	removeDeletePlanTools,
	resolveDeleteTargets
} from './flowDeleteUtils'

function makeRawModule(id: string, expr?: string): FlowModule {
	return {
		id,
		summary: id,
		value: {
			type: 'rawscript',
			content: '',
			language: 'python3',
			input_transforms: expr
				? {
						user_input: {
							type: 'javascript',
							expr
						}
					}
				: {}
		} as any
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

describe('resolveDeleteTargets', () => {
	it('resolves structure nodes, preprocessor, and ai tools while pruning nested descendants', () => {
		const rootAgent = makeAiAgent('root_agent', [
			makeFlowModuleTool(makeRawModule('lookup_user')),
			makeFlowModuleTool(makeAiAgent('support_agent', [makeFlowModuleTool(makeRawModule('create_ticket'))]))
		])
		const tree: FlowStructureNode[] = [{ id: 'root_agent', kind: 'leaf', branches: [] }]

		const { targets, missingIds } = resolveDeleteTargets(
			tree,
			[rootAgent],
			['preprocessor', 'root_agent', 'create_ticket', 'missing_tool'],
			true
		)

		expect(targets.map((target) => target.kind)).toEqual(['preprocessor', 'structure_node'])
		expect(targets[1].stateIds).toEqual([
			'root_agent',
			'lookup_user',
			'support_agent',
			'create_ticket'
		])
		expect(missingIds).toEqual(['missing_tool'])
	})
})

describe('createDeletePlan', () => {
	it('collects subtree dependents and routes structure deletes through the proxy helper', () => {
		const agent = makeAiAgent('agent_step', [makeFlowModuleTool(makeRawModule('lookup_user'))])
		const dependent = makeRawModule('dependent_step', 'results.lookup_user?.value')
		const flow: OpenFlow = {
			summary: 'Flow',
			value: {
				modules: [agent, dependent]
			}
		}
		const tree: FlowStructureNode[] = [
			{ id: 'agent_step', kind: 'leaf', branches: [] },
			{ id: 'dependent_step', kind: 'leaf', branches: [] }
		]
		const commit = vi.fn()
		const prepareDelete = vi.fn(() => ({
			affectedGroups: [],
			duplicateGroups: [],
			commit
		}))
		const proxy = {
			prepareDelete
		} as unknown as GroupedModulesProxy

		const plan = createDeletePlan({
			ids: ['agent_step'],
			flow,
			tree,
			proxy,
			displayState: new GroupDisplayState(() => [])
		})

		expect(prepareDelete).toHaveBeenCalledWith(['agent_step'], expect.any(Object))
		expect(plan?.plannedStateIds).toEqual(['agent_step', 'lookup_user'])
		expect(plan?.structureDelete?.affectedGroups).toEqual([])
		expect(plan?.dependents).toEqual({
			dependent_step: ['results.lookup_user?.value']
		})
		expect(plan?.selection).toEqual({ kind: 'select', id: 'dependent_step' })
	})
})

describe('removeDeletePlanTools', () => {
	it('removes nested tools before their parents and returns every removed id', () => {
		const rootAgent = makeAiAgent('root_agent', [
			makeFlowModuleTool(makeAiAgent('support_agent', [makeFlowModuleTool(makeRawModule('create_ticket'))]))
		])
		const tree: FlowStructureNode[] = []
		const { targets } = resolveDeleteTargets(
			tree,
			[rootAgent],
			['support_agent', 'create_ticket'],
			false
		)

		expect(removeDeletePlanTools(targets, [rootAgent])).toEqual(['support_agent', 'create_ticket'])
		expect((rootAgent.value as any).tools).toEqual([])
	})

	it('re-resolves live owners before removing planned tool targets', () => {
		const rootAgent = makeAiAgent('root_agent', [
			makeFlowModuleTool(makeRawModule('lookup_user')),
			makeFlowModuleTool(makeAiAgent('support_agent', [makeFlowModuleTool(makeRawModule('create_ticket'))]))
		])
		const tree: FlowStructureNode[] = []
		const { targets } = resolveDeleteTargets(tree, [rootAgent], ['support_agent'], false)

		;(rootAgent.value as any).tools.unshift(makeFlowModuleTool(makeRawModule('new_lookup')))

		expect(removeDeletePlanTools(targets, [rootAgent])).toEqual(['support_agent', 'create_ticket'])
		expect(((rootAgent.value as any).tools as any[]).map((tool) => tool.id)).toEqual([
			'new_lookup',
			'lookup_user'
		])
	})
})
