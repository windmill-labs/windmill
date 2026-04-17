import { describe, expect, it, vi } from 'vitest'
import type { FlowModule } from '$lib/gen'
import { applyFlowJsonUpdate, updateRawScriptModuleContent } from './helperUtils'
import { createInlineScriptSession } from './inlineScriptsUtils'

vi.mock('../shared', () => ({
	SPECIAL_MODULE_IDS: {
		PREPROCESSOR: 'preprocessor',
		FAILURE: 'failure'
	}
}))

function makeRawScriptModule(id: string, content: string): FlowModule {
	return {
		id,
		summary: id,
		value: {
			type: 'rawscript',
			language: 'bun',
			content,
			input_transforms: {}
		}
	} as FlowModule
}

function makeAiAgentWithTool(agentId: string, toolId: string, toolContent: string): FlowModule {
	return {
		id: agentId,
		summary: agentId,
		value: {
			type: 'aiagent',
			tools: [
				{
					id: toolId,
					summary: toolId,
					value: {
						tool_type: 'flowmodule',
						type: 'rawscript',
						language: 'bun',
						content: toolContent,
						input_transforms: {}
					}
				}
			],
			input_transforms: {}
		}
	} as FlowModule
}

describe('applyFlowJsonUpdate', () => {
	it('accepts new self-referenced inline scripts and initializes them as empty', () => {
		const flow = {
			value: {
				modules: [makeRawScriptModule('process_data', 'existing code')]
			}
		}
		const inlineScriptSession = createInlineScriptSession()
		inlineScriptSession.set('process_data', 'existing code')

		const result = applyFlowJsonUpdate(flow as any, inlineScriptSession, {
			modules: [
				makeRawScriptModule('process_data', 'inline_script.process_data'),
				makeRawScriptModule('validate_data', 'inline_script.validate_data')
			]
		})
		const [processDataModule, validateDataModule] = flow.value.modules as Array<FlowModule & { value: any }>

		expect(result.emptyInlineScriptModuleIds).toEqual(['validate_data'])
		expect(inlineScriptSession.has('validate_data')).toBe(false)
		expect(processDataModule?.value.type).toBe('rawscript')
		expect(processDataModule?.value.content).toBe('existing code')
		expect(validateDataModule?.value.type).toBe('rawscript')
		expect(validateDataModule?.value.content).toBe('')
	})

	it('still rejects unresolved inline script references that do not match the module id', () => {
		const flow = {
			value: {
				modules: []
			}
		}
		const inlineScriptSession = createInlineScriptSession()

		expect(() =>
			applyFlowJsonUpdate(flow as any, inlineScriptSession, {
				modules: [makeRawScriptModule('validate_data', 'inline_script.other_module')]
			})
			).toThrow('Unresolved inline script references: other_module')
	})

	it('keeps the inline script session unchanged after a failed update so retries still warn', () => {
		const flow = {
			value: {
				modules: [makeRawScriptModule('process_data', 'existing code')]
			}
		}
		const inlineScriptSession = createInlineScriptSession()
		inlineScriptSession.set('process_data', 'existing code')

		expect(() =>
			applyFlowJsonUpdate(flow as any, inlineScriptSession, {
				modules: [
					makeRawScriptModule('validate_data', 'inline_script.validate_data'),
					makeRawScriptModule('save_results', 'inline_script.other_module')
				]
			})
		).toThrow('Unresolved inline script references: other_module')

		expect(inlineScriptSession.getAll()).toEqual({
			process_data: 'existing code'
		})
		expect((flow.value.modules as Array<FlowModule & { value: any }>)[0]?.value.content).toBe(
			'existing code'
		)

		const result = applyFlowJsonUpdate(flow as any, inlineScriptSession, {
			modules: [
				makeRawScriptModule('process_data', 'inline_script.process_data'),
				makeRawScriptModule('validate_data', 'inline_script.validate_data')
			]
		})

		expect(result.emptyInlineScriptModuleIds).toEqual(['validate_data'])
		expect(inlineScriptSession.has('validate_data')).toBe(false)
	})

	it('updates ai agent rawscript tools in place when changing module code', () => {
		const flow = {
			value: {
				modules: [makeAiAgentWithTool('agent', 'sum', '')]
			}
		}

		const updatedModule = updateRawScriptModuleContent(
			flow as any,
			'sum',
			'export async function main(numbers: number[]) { return 0 }'
		)

		expect(updatedModule?.value.content).toBe(
			'export async function main(numbers: number[]) { return 0 }'
		)
		expect((flow.value.modules[0] as any).value.tools[0].value.content).toBe(
			'export async function main(numbers: number[]) { return 0 }'
		)
	})
})
