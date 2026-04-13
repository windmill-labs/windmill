import { describe, expect, it, vi } from 'vitest'

vi.mock('../aiProviderStorage', () => ({
	loadStoredConfig: () => undefined
}))

vi.mock('./flowInfers', () => ({
	AI_AGENT_SCHEMA: { properties: {} }
}))

import { GroupDisplayState } from '$lib/components/graph/groupEditor.svelte'
import {
	GroupedModulesProxy,
	type ExtendedOpenFlow
} from '$lib/components/graph/groupedModulesProxy.svelte'
import type { FlowModule, OpenFlow } from '$lib/gen'
import type { StateStore } from '$lib/utils'
import { executeDeletePlan, prepareDeleteRequest } from './flowDeleteController'

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

describe('flowDeleteController', () => {
	it('prepares confirmations and executes grouped ai-agent deletes end to end', () => {
		const agent = makeAiAgent('agent_step', [
			makeFlowModuleTool(makeRawModule('lookup_user')),
			makeFlowModuleTool(
				makeAiAgent('support_agent', [makeFlowModuleTool(makeRawModule('create_ticket'))])
			)
		])
		const dependent = makeRawModule('dependent_step', 'results.create_ticket?.value')
		const flowStore = {
			val: {
				summary: 'Flow',
				value: {
					modules: [agent, dependent],
					groups: [
						{
							summary: 'Agent group',
							start_id: 'agent_step',
							end_id: 'agent_step'
						}
					]
				}
			} as OpenFlow
		} satisfies StateStore<OpenFlow>
		const proxy = new GroupedModulesProxy(flowStore as unknown as StateStore<ExtendedOpenFlow>)
		const displayState = new GroupDisplayState(() => flowStore.val.value.groups ?? [])

		const request = prepareDeleteRequest({
			ids: ['agent_step'],
			flow: flowStore.val,
			tree: proxy.items,
			proxy,
			displayState
		})

		expect(request?.needsDependencyConfirmation).toBe(true)
		expect(request?.plan.structureDelete?.affectedGroups).toHaveLength(1)
		expect(request?.plan.structureDelete?.affectedGroups[0]).toMatchObject({
			summary: 'Agent group',
			start_id: 'agent_step',
			end_id: 'agent_step'
		})

		const selectionManager = {
			clearSelection: vi.fn(),
			selectId: vi.fn()
		}
		const flowStateStore = {
			val: {
				agent_step: {},
				lookup_user: {},
				support_agent: {},
				create_ticket: {},
				dependent_step: {}
			}
		} as StateStore<any>
		const onDelete = vi.fn()

		const result = executeDeletePlan(request!.plan, {
			flowStore,
			flowStateStore,
			selectionManager,
			onDelete
		})

		expect(result.removedStateIds).toEqual([
			'agent_step',
			'lookup_user',
			'support_agent',
			'create_ticket'
		])
		expect(flowStore.val.value.modules.map((module) => module.id)).toEqual(['dependent_step'])
		expect(flowStore.val.value.groups ?? []).toEqual([])
		expect(Object.keys(flowStateStore.val)).toEqual(['dependent_step'])
		expect(selectionManager.selectId).toHaveBeenCalledWith('dependent_step')
		expect(onDelete).toHaveBeenCalledWith('agent_step')
	})
})
