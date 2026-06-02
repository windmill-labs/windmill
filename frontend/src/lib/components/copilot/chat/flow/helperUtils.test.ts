import { describe, expect, it, vi } from 'vitest'
import type { FlowModule } from '$lib/gen'
import {
	applyFlowJsonUpdate,
	updateRawScriptModuleContent,
	validateFlowGroups,
	validateFlowNotes
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

	it('persists notes passed in the flow json update', () => {
		const flow = {
			value: {
				modules: [makeRawScriptModule('fetch_data', 'existing code')]
			}
		}
		const inlineScriptSession = createInlineScriptSession()
		inlineScriptSession.set('fetch_data', 'existing code')

		applyFlowJsonUpdate(flow as any, inlineScriptSession, {
			notes: [
				{
					id: 'note_1',
					text: 'Remember to set the API key',
					color: 'yellow',
					type: 'free'
				} as any
			]
		})

		expect((flow.value as any).notes).toEqual([
			{
				id: 'note_1',
				text: 'Remember to set the API key',
				color: 'yellow',
				type: 'free'
			}
		])
	})

	it('clears notes when an empty array or null is passed', () => {
		const flow = {
			value: {
				modules: [],
				notes: [{ id: 'n', text: 't', color: 'yellow', type: 'free' }]
			}
		}
		const inlineScriptSession = createInlineScriptSession()

		applyFlowJsonUpdate(flow as any, inlineScriptSession, { notes: [] })
		expect((flow.value as any).notes).toBeUndefined()
		;(flow.value as any).notes = [{ id: 'n', text: 't', color: 'yellow', type: 'free' }]
		applyFlowJsonUpdate(flow as any, inlineScriptSession, { notes: null })
		expect((flow.value as any).notes).toBeUndefined()
	})

	it('leaves notes untouched when not provided in the update', () => {
		const existingNotes = [{ id: 'n', text: 't', color: 'yellow', type: 'free' }]
		const flow = {
			value: {
				modules: [makeRawScriptModule('a', 'existing code')],
				notes: existingNotes
			}
		}
		const inlineScriptSession = createInlineScriptSession()
		inlineScriptSession.set('a', 'existing code')

		applyFlowJsonUpdate(flow as any, inlineScriptSession, {
			modules: [makeRawScriptModule('a', 'inline_script.a')]
		})

		expect((flow.value as any).notes).toEqual(existingNotes)
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

	it('rejects an unknown color name', () => {
		expect(() => validateFlowGroups([{ start_id: 'a', end_id: 'b', color: '#ff00aa' }])).toThrow(
			/color must be one of/
		)
		expect(() => validateFlowGroups([{ start_id: 'a', end_id: 'b', color: 'magenta' }])).toThrow(
			/color must be one of/
		)
	})

	it('accepts a known color name', () => {
		const result = validateFlowGroups([{ start_id: 'a', end_id: 'b', color: 'blue' }])
		expect(result).toEqual([{ start_id: 'a', end_id: 'b', color: 'blue' }])
	})

	it('accepts a group with no color', () => {
		const result = validateFlowGroups([{ start_id: 'a', end_id: 'b' }])
		expect(result).toEqual([{ start_id: 'a', end_id: 'b' }])
	})
})

describe('validateFlowNotes', () => {
	it('returns null for null/undefined input', () => {
		expect(validateFlowNotes(null)).toBeNull()
		expect(validateFlowNotes(undefined)).toBeNull()
	})

	it('rejects non-array input', () => {
		expect(() => validateFlowNotes({})).toThrow('Flow notes must be an array')
	})

	it('rejects a note that is not an object', () => {
		expect(() => validateFlowNotes(['nope'])).toThrow('Invalid note at index 0: must be an object')
	})

	it('rejects a note with a missing or non-string id', () => {
		expect(() => validateFlowNotes([{ text: 't' }])).toThrow(
			'Invalid note at index 0: id must be a non-empty string'
		)
		expect(() => validateFlowNotes([{ id: '', text: 't' }])).toThrow(
			'Invalid note at index 0: id must be a non-empty string'
		)
	})

	it('rejects duplicate note ids', () => {
		expect(() =>
			validateFlowNotes([
				{ id: 'n', text: 'a' },
				{ id: 'n', text: 'b' }
			])
		).toThrow('Invalid note at index 1: duplicate note id "n"')
	})

	it('rejects a non-string text', () => {
		expect(() => validateFlowNotes([{ id: 'n', text: 42 }])).toThrow(
			'Invalid note at index 0: text must be a string'
		)
	})

	it('rejects an invalid type', () => {
		expect(() => validateFlowNotes([{ id: 'n', text: 't', type: 'sticky' }])).toThrow(
			'Invalid note at index 0: type must be "free" or "group"'
		)
	})

	it('rejects an unknown color name', () => {
		expect(() => validateFlowNotes([{ id: 'n', text: 't', color: '#ffff00' }])).toThrow(
			/color must be one of/
		)
	})

	it('defaults type to free and color to the default note color', () => {
		const [note] = validateFlowNotes([{ id: 'n', text: 't' }])!
		expect(note).toMatchObject({ id: 'n', text: 't', type: 'free', color: 'green' })
	})

	it('preserves a provided palette color rather than overriding it', () => {
		const [note] = validateFlowNotes([{ id: 'n', text: 't', color: 'purple' }])!
		expect(note).toMatchObject({ id: 'n', text: 't', type: 'free', color: 'purple' })
	})

	it('gives a free note missing geometry a concrete position and size', () => {
		const [note] = validateFlowNotes([{ id: 'n', text: 't' }])!
		expect(note.position).toEqual({ x: expect.any(Number), y: expect.any(Number) })
		expect(note.size).toEqual({ width: expect.any(Number), height: expect.any(Number) })
		expect(note.size!.width).toBeGreaterThan(0)
		expect(note.size!.height).toBeGreaterThan(0)
	})

	it('staggers the default y position of multiple geometry-less free notes', () => {
		const notes = validateFlowNotes([
			{ id: 'a', text: 't' },
			{ id: 'b', text: 't' }
		])!
		expect(notes[0].position!.y).not.toEqual(notes[1].position!.y)
	})

	it('does not override a free note that already has geometry', () => {
		const [note] = validateFlowNotes([
			{ id: 'n', text: 't', position: { x: 5, y: 6 }, size: { width: 400, height: 90 } }
		])!
		expect(note.position).toEqual({ x: 5, y: 6 })
		expect(note.size).toEqual({ width: 400, height: 90 })
	})

	it('does not add geometry to group notes', () => {
		const [note] = validateFlowNotes([
			{ id: 'n', text: 't', type: 'group', contained_node_ids: [] }
		])!
		expect(note.position).toBeUndefined()
		expect(note.size).toBeUndefined()
	})

	it('rejects a malformed position', () => {
		expect(() => validateFlowNotes([{ id: 'n', text: 't', position: { x: 1 } }])).toThrow(
			'Invalid note at index 0: position must be an object with numeric x and y'
		)
		expect(() => validateFlowNotes([{ id: 'n', text: 't', position: [1, 2] }])).toThrow(
			'Invalid note at index 0: position must be an object with numeric x and y'
		)
	})

	it('rejects a malformed size', () => {
		expect(() => validateFlowNotes([{ id: 'n', text: 't', size: { width: '10' } }])).toThrow(
			'Invalid note at index 0: size must be an object with numeric width and height'
		)
	})

	it('accepts a free note with valid position and size', () => {
		const result = validateFlowNotes([
			{
				id: 'n',
				text: 't',
				color: 'blue',
				position: { x: 10, y: 20 },
				size: { width: 300, height: 80 }
			}
		])
		expect(result).toEqual([
			{
				id: 'n',
				text: 't',
				type: 'free',
				color: 'blue',
				position: { x: 10, y: 20 },
				size: { width: 300, height: 80 }
			}
		])
	})

	it('rejects group note contained_node_ids that are not strings', () => {
		expect(() =>
			validateFlowNotes([{ id: 'n', text: 't', type: 'group', contained_node_ids: [1] }])
		).toThrow('Invalid note at index 0: contained_node_ids must be an array of strings')
	})

	it('rejects group note contained_node_ids that do not match a module', () => {
		const moduleIds = new Set(['a', 'b'])
		expect(() =>
			validateFlowNotes(
				[{ id: 'n', text: 't', type: 'group', contained_node_ids: ['missing'] }],
				moduleIds
			)
		).toThrow(
			'Invalid note at index 0: contained_node_ids "missing" does not match any flow module'
		)
	})

	it('accepts a valid group note whose contained ids are all modules', () => {
		const moduleIds = new Set(['a', 'b'])
		const result = validateFlowNotes(
			[{ id: 'n', text: 't', color: 'blue', type: 'group', contained_node_ids: ['a', 'b'] }],
			moduleIds
		)
		expect(result).toEqual([
			{ id: 'n', text: 't', color: 'blue', type: 'group', contained_node_ids: ['a', 'b'] }
		])
	})
})
