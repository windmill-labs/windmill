import { describe, expect, it } from 'vitest'
import type { FlowModule } from '$lib/gen'
import { collectAllFlowModuleIdsFromModules } from '../../../../components/flows/flowTree'

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
	it('collects nested module ids and all ai agent tool ids for flow-json validation', () => {
		const modules = [
			{
				id: 'router',
				value: {
					type: 'branchone',
					default: [{ id: 'default_step', value: { type: 'identity' } }],
					branches: [{ expr: 'true', modules: [{ id: 'branch_step', value: { type: 'identity' } }] }],
					input_transforms: {}
				}
			} as any as FlowModule,
			createAiAgentModule('agent', [
				createAiAgentTool('lookup', 'lookup-tool'),
				{
					id: 'search_docs',
					summary: 'search_docs',
					value: {
						tool_type: 'mcp',
						resource_path: ''
					}
				} as any
			])
		]

		expect(collectAllFlowModuleIdsFromModules(modules)).toEqual([
			'router',
			'branch_step',
			'default_step',
			'agent',
			'lookup',
			'search_docs'
		])
	})
})
