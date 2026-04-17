import { describe, expect, it } from 'vitest'
import type { FlowModule, OpenFlow } from '$lib/gen'
import { getIndexInNestedModules, getModuleById } from './utils'

function createAiAgentTool(id: string, content: string) {
	return {
		id,
		summary: id,
		value: {
			tool_type: 'flowmodule',
			type: 'rawscript',
			content,
			language: 'bun',
			input_transforms: {}
		}
	}
}

function createAiAgentModule(id: string, tools: ReturnType<typeof createAiAgentTool>[]): FlowModule {
	return {
		id,
		value: {
			type: 'aiagent',
			tools,
			input_transforms: {}
		} as any
	}
}

describe('chat flow utils', () => {
	it('resolves ai agent tools by id against the stored flow node', () => {
		const flow = {
			value: {
				modules: [createAiAgentModule('agent', [createAiAgentTool('sum', 'before-tool')])]
			}
		} as OpenFlow

		const tool = getModuleById(flow, 'sum')

		expect(tool?.id).toBe('sum')
		expect((tool?.value as any).content).toBe('before-tool')

		;(tool?.value as any).content = 'after-tool'
		expect(((flow.value.modules[0].value as any).tools[0].value as any).content).toBe('after-tool')
	})

	it('returns the ai agent tools array for nested index operations', () => {
		const flow = {
			value: {
				modules: [
					createAiAgentModule('agent', [
						createAiAgentTool('lookup', 'lookup-tool'),
						createAiAgentTool('sum', 'sum-tool')
					])
				]
			}
		} as OpenFlow

		const result = getIndexInNestedModules(flow, 'sum')

		expect(result).not.toBeNull()
		expect(result?.index).toBe(1)
		expect(result?.modules.map((module) => module.id)).toEqual(['lookup', 'sum'])

		result?.modules.splice(result.index, 1)
		expect(((flow.value.modules[0].value as any).tools ?? []).map((tool: any) => tool.id)).toEqual([
			'lookup'
		])
	})
})
