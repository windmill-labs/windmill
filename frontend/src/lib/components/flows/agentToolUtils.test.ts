import { describe, expect, it, vi } from 'vitest'

vi.mock('../aiProviderStorage', () => ({
	loadStoredConfig: () => undefined
}))

vi.mock('./flowInfers', () => ({
	AI_AGENT_SCHEMA: { properties: {} }
}))

import type { FlowModule } from '$lib/gen'
import { removeAgentToolByIdDeep } from './agentToolUtils'

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

describe('removeAgentToolByIdDeep', () => {
	it('removes a direct tool from an ai agent', () => {
		const tool = makeFlowModuleTool(makeRawModule('lookup_user'))
		const agent = makeAiAgent('agent', [tool])
		const removed: string[] = []

		expect(removeAgentToolByIdDeep([agent], 'lookup_user', (x) => removed.push(x.id))).toBe(true)
		expect((agent.value as any).tools).toEqual([])
		expect(removed).toEqual(['lookup_user'])
	})

	it('removes a nested tool from a nested ai agent tool', () => {
		const nestedTool = makeFlowModuleTool(makeRawModule('create_ticket'))
		const nestedAgent = makeAiAgent('support_agent', [nestedTool])
		const rootAgent = makeAiAgent('root_agent', [makeFlowModuleTool(nestedAgent)])
		const removed: string[] = []

		expect(removeAgentToolByIdDeep([rootAgent], 'create_ticket', (x) => removed.push(x.id))).toBe(
			true
		)
		expect((((rootAgent.value as any).tools as any[])[0].value as any).tools).toEqual([])
		expect(removed).toEqual(['create_ticket'])
	})
})
