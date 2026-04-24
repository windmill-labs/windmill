import { describe, expect, it, vi } from 'vitest'
import type { FlowModule } from '$lib/gen'
import {
	applyFlowJsonUpdate,
	updateRawScriptModuleContent,
	validateFlowGroups
} from './helperUtils'
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
		const [processDataModule, validateDataModule] = flow.value.modules as Array<
			FlowModule & { value: any }
		>

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

	it('persists groups passed in the flow json update', () => {
		const flow = {
			value: {
				modules: [
					makeRawScriptModule('fetch_data', 'existing code'),
					makeRawScriptModule('process_data', 'existing code')
				]
			}
		}
		const inlineScriptSession = createInlineScriptSession()
		inlineScriptSession.set('fetch_data', 'existing code')
		inlineScriptSession.set('process_data', 'existing code')

		applyFlowJsonUpdate(flow as any, inlineScriptSession, {
			groups: [
				{
					summary: 'Data Ingestion',
					note: 'Fetches and processes data',
					start_id: 'fetch_data',
					end_id: 'process_data'
				}
			]
		})

		expect((flow.value as any).groups).toEqual([
			{
				summary: 'Data Ingestion',
				note: 'Fetches and processes data',
				start_id: 'fetch_data',
				end_id: 'process_data'
			}
		])
	})

	it('clears groups when an empty array is passed', () => {
		const flow = {
			value: {
				modules: [],
				groups: [
					{
						summary: 'existing',
						start_id: 'a',
						end_id: 'b'
					}
				]
			}
		}
		const inlineScriptSession = createInlineScriptSession()

		applyFlowJsonUpdate(flow as any, inlineScriptSession, { groups: [] })

		expect((flow.value as any).groups).toBeUndefined()
	})

	it('clears groups when null is passed', () => {
		const flow = {
			value: {
				modules: [],
				groups: [
					{
						summary: 'existing',
						start_id: 'a',
						end_id: 'b'
					}
				]
			}
		}
		const inlineScriptSession = createInlineScriptSession()

		applyFlowJsonUpdate(flow as any, inlineScriptSession, { groups: null })

		expect((flow.value as any).groups).toBeUndefined()
	})

	it('leaves groups untouched when not provided in the update', () => {
		const existingGroups = [{ summary: 'existing', start_id: 'a', end_id: 'b' }]
		const flow = {
			value: {
				modules: [makeRawScriptModule('a', 'existing code')],
				groups: existingGroups
			}
		}
		const inlineScriptSession = createInlineScriptSession()
		inlineScriptSession.set('a', 'existing code')

		applyFlowJsonUpdate(flow as any, inlineScriptSession, {
			modules: [makeRawScriptModule('a', 'inline_script.a')]
		})

		expect((flow.value as any).groups).toEqual(existingGroups)
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

describe('validateFlowGroups', () => {
	it('returns null for null input', () => {
		expect(validateFlowGroups(null)).toBeNull()
		expect(validateFlowGroups(undefined)).toBeNull()
	})

	it('rejects non-array input', () => {
		expect(() => validateFlowGroups({})).toThrow('Flow groups must be an array')
		expect(() => validateFlowGroups('not an array')).toThrow('Flow groups must be an array')
	})

	it('rejects a group that is not an object', () => {
		expect(() => validateFlowGroups(['nope'])).toThrow(
			'Invalid group at index 0: must be an object'
		)
	})

	it('rejects a group with a missing or non-string start_id', () => {
		expect(() => validateFlowGroups([{ end_id: 'b' }])).toThrow(
			'Invalid group at index 0: start_id must be a non-empty string'
		)
		expect(() => validateFlowGroups([{ start_id: '', end_id: 'b' }])).toThrow(
			'Invalid group at index 0: start_id must be a non-empty string'
		)
		expect(() => validateFlowGroups([{ start_id: 42, end_id: 'b' }])).toThrow(
			'Invalid group at index 0: start_id must be a non-empty string'
		)
	})

	it('rejects a group with a missing end_id', () => {
		expect(() => validateFlowGroups([{ start_id: 'a' }])).toThrow(
			'Invalid group at index 0: end_id must be a non-empty string'
		)
	})

	it('accepts a valid group with no moduleIds set', () => {
		const result = validateFlowGroups([{ summary: 'G', start_id: 'a', end_id: 'b' }])
		expect(result).toEqual([{ summary: 'G', start_id: 'a', end_id: 'b' }])
	})

	it('rejects start_id or end_id that are not in the moduleIds set', () => {
		const moduleIds = new Set(['a', 'b'])
		expect(() => validateFlowGroups([{ start_id: 'missing', end_id: 'b' }], moduleIds)).toThrow(
			'Invalid group at index 0: start_id "missing" does not match any flow module'
		)
		expect(() => validateFlowGroups([{ start_id: 'a', end_id: 'missing' }], moduleIds)).toThrow(
			'Invalid group at index 0: end_id "missing" does not match any flow module'
		)
	})

	it('accepts groups whose ids are all in the moduleIds set', () => {
		const moduleIds = new Set(['a', 'b', 'c'])
		const result = validateFlowGroups([{ start_id: 'a', end_id: 'c', summary: 'G' }], moduleIds)
		expect(result).toEqual([{ start_id: 'a', end_id: 'c', summary: 'G' }])
	})
})
