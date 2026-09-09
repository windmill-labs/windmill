import { beforeEach, describe, expect, it, vi } from 'vitest'

import {
	inlineAgentDraft,
	inlineAgentDrafts,
	loadLinkedAgentDrafts,
	type LinkedAgentDraft
} from './linkedAgentDrafts'
import { ResourceService, type FlowModule, type FlowValue } from '$lib/gen'

type AiAgentValue = Extract<FlowModule['value'], { type: 'aiagent' }>

function linkedStep(input_transforms: Record<string, any>): AiAgentValue {
	return {
		type: 'aiagent',
		agent: 'f/team/support',
		tool_inputs: { t1: { query: { type: 'javascript', expr: 'flow_input.q' } } },
		input_transforms
	} as unknown as AiAgentValue
}

describe('inlineAgentDraft', () => {
	// The overlay order is the worker's: the resource brain first, the step's flow-local inputs on
	// top. Reversing it would run the agent author's own `user_message` instead of the flow's.
	it('keeps the step wired to the flow while the brain comes from the draft', () => {
		const inlined = inlineAgentDraft(
			linkedStep({
				user_message: { type: 'javascript', expr: 'flow_input.question' },
				user_attachments: { type: 'static', value: [] }
			}),
			{
				provider: { kind: 'openai', model: 'gpt-4o', resource: '$res:f/team/openai' },
				system_prompt: 'answer in french',
				user_message: 'the default the agent carries',
				tools: [{ id: 't1', summary: 'search' }]
			} as any
		)

		expect(inlined.agent).toBeUndefined()
		expect(inlined.tools).toEqual([{ id: 't1', summary: 'search' }])
		expect(inlined.input_transforms).toEqual({
			provider: {
				type: 'static',
				value: { kind: 'openai', model: 'gpt-4o', resource: '$res:f/team/openai' }
			},
			system_prompt: { type: 'static', value: 'answer in french' },
			user_message: { type: 'javascript', expr: 'flow_input.question' },
			user_attachments: { type: 'static', value: [] }
		})
		// Host bindings are the step's, not the agent's, and the worker overlays them either way.
		expect(inlined.tool_inputs).toEqual({
			t1: { query: { type: 'javascript', expr: 'flow_input.q' } }
		})
	})

	// A linked step carries only the flow-local inputs, but one persisted before linking existed can
	// still hold stale brain transforms. They must not shadow the draft the test is meant to run.
	it('drops brain transforms the step still carries', () => {
		const inlined = inlineAgentDraft(
			linkedStep({
				user_message: { type: 'static', value: 'hi' },
				system_prompt: { type: 'static', value: 'stale' }
			}),
			{ system_prompt: 'from the draft' } as any
		)

		expect(inlined.input_transforms).toEqual({
			system_prompt: { type: 'static', value: 'from the draft' },
			user_message: { type: 'static', value: 'hi' }
		})
	})

	// Every step carries a placeholder transform for each field its form has not filled in. It reads
	// as an override unless it is recognised as unset, which would run a preview without the memory
	// an agent saved before memory moved onto the step still carries — and a deployed run with it.
	it('lets an unfilled flow-local field inherit from the draft', () => {
		const inlined = inlineAgentDraft(
			linkedStep({
				user_message: { type: 'static', value: 'hi' },
				memory: { type: 'static', value: undefined }
			}),
			{ memory: { kind: 'auto', context_length: 20 } } as any
		)

		expect(inlined.input_transforms?.memory).toEqual({
			type: 'static',
			value: { kind: 'auto', context_length: 20 }
		})
	})

	it('lets a filled flow-local field override the draft', () => {
		const inlined = inlineAgentDraft(
			linkedStep({
				user_message: { type: 'static', value: 'hi' },
				memory: { type: 'static', value: { kind: 'off' } }
			}),
			{ memory: { kind: 'auto', context_length: 20 } } as any
		)

		expect(inlined.input_transforms?.memory).toEqual({
			type: 'static',
			value: { kind: 'off' }
		})
	})
})

describe('inlineAgentDrafts', () => {
	// The index is keyed on the bare path while a step may name its agent `$res:`-prefixed, and the
	// walk has to reach inside branches and loops. Miss either and every preview silently runs the
	// deployed agent — the failure this whole path exists to prevent, and a silent one.
	it('reaches a $res:-prefixed link nested in a branch', () => {
		const value = {
			modules: [
				{
					id: 'b',
					value: {
						type: 'branchone',
						default: [],
						branches: [
							{
								modules: [
									{
										id: 'inner',
										value: {
											type: 'aiagent',
											agent: '$res:f/team/support',
											tools: [],
											input_transforms: { user_message: { type: 'static', value: 'hi' } }
										}
									}
								]
							}
						]
					}
				}
			]
		} as unknown as FlowValue

		const drafts = new Map<string, LinkedAgentDraft>([
			['f/team/support', { args: { system_prompt: 'drafted' } } as unknown as LinkedAgentDraft]
		])

		const inner = (inlineAgentDrafts(value, drafts).modules[0].value as any).branches[0].modules[0]
		expect(inner.value.agent).toBeUndefined()
		expect(inner.value.input_transforms.system_prompt).toEqual({
			type: 'static',
			value: 'drafted'
		})
		// The input the flow supplies survives the rewrite.
		expect(inner.value.input_transforms.user_message).toEqual({ type: 'static', value: 'hi' })
	})
})

// A link the user cannot resolve is an ordinary state and must not block the flow; anything else is
// an outage, and answering "no draft" to one would silently test or deploy against the deployed
// agent while the editor shows the draft.
describe('loadLinkedAgentDrafts error handling', () => {
	function failWith(status: number | undefined) {
		return async () => {
			const err: Error & { status?: number } = new Error('boom')
			err.status = status
			throw err
		}
	}

	beforeEach(() => {
		vi.restoreAllMocks()
	})

	it.each([401, 403, 404])('treats %i as no draft', async (status) => {
		vi.spyOn(ResourceService, 'getResource').mockImplementation(failWith(status) as any)
		await expect(loadLinkedAgentDrafts(['f/team/support'], 'ws')).resolves.toEqual(new Map())
	})

	it.each([500, undefined])('propagates %s rather than reporting no draft', async (status) => {
		vi.spyOn(ResourceService, 'getResource').mockImplementation(failWith(status) as any)
		await expect(loadLinkedAgentDrafts(['f/team/support'], 'ws')).rejects.toThrow(
			'Could not load the agent f/team/support'
		)
	})
})
