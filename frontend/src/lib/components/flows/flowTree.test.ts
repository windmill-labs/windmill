import { describe, expect, it } from 'vitest'
import type { FlowModule, FlowValue } from '$lib/gen'
import {
	collectAllFlowModuleIds,
	collectAllFlowModuleIdsFromModules,
	collectFlowNodes,
	ensureModuleArrayByLocation,
	findFlowNode,
	findModuleInFlow,
	findModuleInModules,
	findModuleParent,
	getModuleArrayContainer,
	removeFlowModule,
	replaceFlowModule
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

function makeMcpTool(id: string) {
	return {
		id,
		summary: id,
		value: {
			tool_type: 'mcp',
			resource_path: ''
		}
	}
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

	it('finds ai agent flowmodule tools inside a plain modules array', () => {
		const tool = makeFlowModuleTool(makeRawModule('sum'))
		const flowModules = [makeAiAgent('agent', [tool])]

		expect(findModuleInModules(flowModules, 'sum')).toBe(tool)
	})

	it('skips non-flowmodule ai agent tools in module traversal', () => {
		const lookup = makeFlowModuleTool(makeRawModule('lookup_user'))
		const agent = makeAiAgent('agent', [lookup, makeMcpTool('search_docs') as any])
		const flow: FlowValue = {
			modules: [agent]
		}

		expect(findModuleInModules(flow.modules ?? [], 'search_docs')).toBeUndefined()
		expect(findModuleInFlow(flow, 'search_docs')).toBeNull()
		expect(collectFlowNodes(flow).map((entry) => entry.module.id)).toEqual(['agent', 'lookup_user'])
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
	expect(findModuleInFlow(flow, 'failure')).toBe(failure)
	expect(findModuleInFlow(flow, 'preprocessor')).toBe(preprocessor)
	expect(getModuleArrayContainer(flow, 'failure')).toBeNull()
	expect(getModuleArrayContainer(flow, 'preprocessor')).toBeNull()
})

	it('removes nested ai agent tools from the real stored tools array', () => {
		const lookup = makeFlowModuleTool(makeRawModule('lookup_user'))
		const sum = makeFlowModuleTool(makeRawModule('sum'))
		const agent = makeAiAgent('agent', [lookup, sum])
		const flow: FlowValue = {
			modules: [agent]
		}

		const removed = removeFlowModule(flow, 'sum')

		expect(removed?.id).toBe('sum')
		expect((((agent.value as any).tools ?? []) as FlowModule[]).map((module) => module.id)).toEqual([
			'lookup_user'
		])
	})

	it('replaces module content while preserving the original object reference', () => {
		const original = makeRawModule('sum')
		const replacement = makeRawModule('sum')
		;(replacement.value as any).content = 'updated'

		const result = replaceFlowModule(original, replacement)

		expect(result).toBe(original)
		expect((original.value as any).content).toBe('updated')
	})

	it('collectAllFlowModuleIds includes all aiagent tool IDs including non-flowmodule', () => {
		const lookup = makeFlowModuleTool(makeRawModule('lookup_user'))
		const mcp = makeMcpTool('search_docs') as any
		const agent = makeAiAgent('agent', [lookup, mcp])
		const failure = makeRawModule('failure')
		const flow: FlowValue = {
			modules: [makeRawModule('step_a'), agent],
			failure_module: failure
		}

		const ids = collectAllFlowModuleIds(flow)

		// Should include ALL tool IDs (both flowmodule and MCP)
		expect(ids).toContain('step_a')
		expect(ids).toContain('agent')
		expect(ids).toContain('lookup_user')
		expect(ids).toContain('search_docs')
		expect(ids).toContain('failure')
		expect(ids).toHaveLength(5)
	})

	it('collectAllFlowModuleIdsFromModules works without special modules', () => {
		const agent = makeAiAgent('agent', [
			makeFlowModuleTool(makeRawModule('sum')),
			makeMcpTool('websearch') as any
		])

		const ids = collectAllFlowModuleIdsFromModules([agent])

		expect(ids).toEqual(['agent', 'sum', 'websearch'])
	})

	it('collectFlowNodes excludes non-flowmodule tools (MCP)', () => {
		// Verify that collectFlowNodes still filters non-flowmodule tools
		const lookup = makeFlowModuleTool(makeRawModule('lookup_user'))
		const mcp = makeMcpTool('search_docs') as any
		const agent = makeAiAgent('agent', [lookup, mcp])
		const flow: FlowValue = { modules: [agent] }

		const nodeIds = collectFlowNodes(flow).map((n) => n.module.id)

		expect(nodeIds).toContain('agent')
		expect(nodeIds).toContain('lookup_user')
		expect(nodeIds).not.toContain('search_docs')
	})

	it('recreates a missing branch container from the source flow location', () => {
		const nested = makeRawModule('nested')
		const beforeFlow: FlowValue = {
			modules: [
				{
					id: 'router',
					summary: 'router',
					value: {
						type: 'branchone',
						default: [],
						branches: [{ expr: 'true', modules: [nested] }],
						input_transforms: {}
					} as any
				}
			]
		}
		const targetFlow: FlowValue = {
			modules: [
				{
					id: 'router',
					summary: 'router',
					value: {
						type: 'branchone',
						default: [],
						branches: [],
						input_transforms: {}
					} as any
				}
			]
		}

		const recreatedContainer = ensureModuleArrayByLocation(
			targetFlow,
			{
				type: 'branchone-branch',
				parentId: 'router',
				branchIndex: 0,
				index: 0
			},
			beforeFlow
		)

		expect(recreatedContainer).toEqual([])
		expect((((targetFlow.modules?.[0].value as any).branches ?? []) as any[])[0]).toMatchObject({
			expr: 'true',
			modules: []
		})
	})
})
