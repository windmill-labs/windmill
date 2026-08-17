import { describe, expect, it } from 'vitest'
import type { FlowModule, OpenFlow } from '$lib/gen'
import { dfsByModule, getPreviousIds, getPreviousModule } from './previousResults'

function makeRawModule(id: string): FlowModule {
	return {
		id,
		summary: id,
		value: {
			type: 'rawscript',
			content: '',
			language: 'python3',
			input_transforms: {}
		} as any
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

describe('previousResults', () => {
	it('returns real ai agent tools when walking parent chains', () => {
		const setup = makeRawModule('setup')
		const lookup = makeFlowModuleTool(makeRawModule('lookup_user'))
		const sum = makeFlowModuleTool(makeRawModule('sum'))
		const agent = makeAiAgent('agent', [lookup, sum])

		const path = dfsByModule('sum', [setup, agent])

		expect(path).toEqual([sum, agent])
		expect(path[0]).toBe(sum)
	})

	it('keeps previous-id ordering for ai agent flowmodule tools', () => {
		const setup = makeRawModule('setup')
		const lookup = makeFlowModuleTool(makeRawModule('lookup_user'))
		const sum = makeFlowModuleTool(makeRawModule('sum'))
		const agent = makeAiAgent('agent', [lookup, sum])
		const flow: OpenFlow = {
			summary: 'Flow',
			value: {
				modules: [setup, agent]
			}
		}

		expect(dfsByModule('sum', flow.value.modules, false).map((module) => module.id)).toEqual([
			'sum',
			'lookup_user',
			'setup'
		])
		expect(getPreviousIds('sum', flow, false)).toEqual(['lookup_user', 'setup'])
		expect(getPreviousModule('sum', flow)?.id).toBe('lookup_user')
	})
})
