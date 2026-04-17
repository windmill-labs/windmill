import { describe, expect, it } from 'vitest'
import type { FlowModule, FlowValue } from '$lib/gen'
import {
	findFlowNode,
	findModuleInFlow,
	findModuleParent,
	getModuleArrayContainer
} from './flowTree'

function makeRawModule(id: string): FlowModule {
	return {
		id,
		summary: id,
		value: { type: 'rawscript', content: '', language: 'python3', input_transforms: {} } as any
	} as FlowModule
}

function makeFlowModuleTool(module: FlowModule): FlowModule {
	return {
		id: module.id,
		summary: module.summary,
		value: {
			tool_type: 'flowmodule',
			...module.value
		} as any
	} as FlowModule
}

function makeAiAgent(id: string, tools: FlowModule[]): FlowModule {
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

describe('flowTree', () => {
	it('returns the root container for top-level modules', () => {
		const first = makeRawModule('first')
		const second = makeRawModule('second')
		const flow: FlowValue = {
			modules: [first, second]
		}

		expect(findModuleParent(flow, 'second')).toEqual({ type: 'root', index: 1 })
		expect(findModuleInFlow(flow, 'second')).toBe(second)
		expect(getModuleArrayContainer(flow, 'second')).toEqual({
			index: 1,
			modules: flow.modules
		})
	})

	it('returns nested branch containers from the live flow tree', () => {
		const nested = makeRawModule('nested')
		const branch = makeRawModule('branch')
		const flow: FlowValue = {
			modules: [
				{
					id: 'router',
					summary: 'router',
					value: {
						type: 'branchone',
						default: [nested],
						branches: [{ expr: 'true', modules: [branch] }],
						input_transforms: {}
					} as any
				}
			]
		}

		expect(findModuleParent(flow, 'nested')).toEqual({
			type: 'branchone-default',
			parentId: 'router',
			index: 0
		})
		expect(getModuleArrayContainer(flow, 'branch')).toEqual({
			index: 0,
			modules: ((flow.modules?.[0].value as any).branches[0].modules ?? []) as FlowModule[]
		})
	})

	it('returns ai agent tool containers as mutable flow module arrays', () => {
		const lookup = makeFlowModuleTool(makeRawModule('lookup_user'))
		const sum = makeFlowModuleTool(makeRawModule('sum'))
		const agent = makeAiAgent('agent', [lookup, sum])
		const agentTools = ((agent.value as any).tools ?? []) as FlowModule[]
		const flow: FlowValue = {
			modules: [agent]
		}

		const match = findFlowNode(flow, 'sum')
		const container = getModuleArrayContainer(flow, 'sum')

		expect(match).toMatchObject({
			location: { type: 'aiagent', parentId: 'agent', index: 1 }
		})
		expect(findModuleInFlow(flow, 'sum')).toBe(sum)
		expect(container).toEqual({
			index: 1,
			modules: agentTools
		})

		container?.modules.splice(container.index, 1)
		expect(agentTools.map((module) => module.id)).toEqual(['lookup_user'])
	})

	it('returns special modules without pretending they belong to an array container', () => {
		const failure = makeRawModule('failure')
		const preprocessor = makeRawModule('preprocessor')
		const flow: FlowValue = {
			modules: [],
			failure_module: failure,
			preprocessor_module: preprocessor
		}

		expect(findModuleParent(flow, 'failure')).toEqual({ type: 'failure', index: -1 })
		expect(findModuleParent(flow, 'preprocessor')).toEqual({
			type: 'preprocessor',
			index: -1
		})
		expect(getModuleArrayContainer(flow, 'failure')).toBeNull()
		expect(getModuleArrayContainer(flow, 'preprocessor')).toBeNull()
	})
})
