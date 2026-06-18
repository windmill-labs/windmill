import { describe, expect, it, vi } from 'vitest'

vi.mock('../aiProviderStorage', () => ({
	loadStoredConfig: () => undefined
}))

vi.mock('./flowInfers', () => ({
	AI_AGENT_SCHEMA: { properties: {} }
}))

import type { FlowModule } from '$lib/gen'
import {
	collectFlowNodeIds,
	findAgentToolOwner,
	removeAgentToolOwner
} from './agentToolTree'

function makeRawModule(id: string): FlowModule {
	return {
		id,
		summary: id,
		value: { type: 'rawscript', content: '', language: 'python3', input_transforms: {} } as any
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

describe('findAgentToolOwner', () => {
	it('finds a direct tool owner in an ai agent', () => {
		const rootAgent = makeAiAgent('root_agent', [makeFlowModuleTool(makeRawModule('lookup_user'))])

		expect(findAgentToolOwner([rootAgent], 'lookup_user')).toMatchObject({
			agentId: 'root_agent',
			toolIndex: 0,
			depth: 1
		})
	})

	it('finds a nested tool owner inside a nested ai agent tool', () => {
		const nestedAgent = makeAiAgent('support_agent', [makeFlowModuleTool(makeRawModule('create_ticket'))])
		const rootAgent = makeAiAgent('root_agent', [makeFlowModuleTool(nestedAgent)])

		expect(findAgentToolOwner([rootAgent], 'create_ticket')).toMatchObject({
			agentId: 'support_agent',
			toolIndex: 0,
			depth: 2
		})
	})
})

describe('removeAgentToolOwner', () => {
	it('removes the matched tool and returns its subtree ids', () => {
		const nestedAgent = makeAiAgent('support_agent', [makeFlowModuleTool(makeRawModule('create_ticket'))])
		const rootAgent = makeAiAgent('root_agent', [
			makeFlowModuleTool(makeRawModule('lookup_user')),
			makeFlowModuleTool(nestedAgent)
		])
		const owner = findAgentToolOwner([rootAgent], 'support_agent')

		expect(owner).toBeDefined()
		expect(removeAgentToolOwner(owner!)).toEqual({
			tool: expect.objectContaining({ id: 'support_agent' }),
			removedIds: ['support_agent', 'create_ticket']
		})
		expect((rootAgent.value as any).tools).toHaveLength(1)
		expect(((rootAgent.value as any).tools as any[]).map((tool) => tool.id)).toEqual(['lookup_user'])
	})
})

describe('collectFlowNodeIds', () => {
	it('includes ai agent tool ids when deleting an ai agent flow module', () => {
		const agent = makeAiAgent('root_agent', [
			makeFlowModuleTool(makeRawModule('lookup_user')),
			makeFlowModuleTool(makeAiAgent('support_agent', [makeFlowModuleTool(makeRawModule('create_ticket'))]))
		])

		expect(collectFlowNodeIds(agent)).toEqual([
			'root_agent',
			'lookup_user',
			'support_agent',
			'create_ticket'
		])
	})
})
