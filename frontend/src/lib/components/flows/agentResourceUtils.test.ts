import { describe, expect, it } from 'vitest'

import {
	agentConfigToInputTransforms,
	flowLocalInputs,
	inputTransformsToAgentConfig,
	nonStaticBrainKeys,
	summarizeAgentBrain
} from './agentResourceUtils'

describe('summarizeAgentBrain', () => {
	it('returns only set fields, in brain-key order, formatted', () => {
		const rows = summarizeAgentBrain({
			provider: { kind: 'openai', model: 'gpt-4o', resource: '$res:f/x/openai' } as any,
			system_prompt: 'You are helpful',
			temperature: 0.7,
			streaming: true,
			max_iterations: 10
		})
		expect(rows).toEqual([
			{ label: 'Model', value: 'openai · gpt-4o' },
			{ label: 'System prompt', value: 'You are helpful' },
			{ label: 'Streaming', value: 'on' },
			{ label: 'Temperature', value: '0.7' },
			{ label: 'Max iterations', value: '10' }
		])
	})

	it('skips empty/undefined fields', () => {
		expect(summarizeAgentBrain({ system_prompt: '', provider: undefined as any })).toEqual([])
		expect(summarizeAgentBrain(undefined)).toEqual([])
	})

	it('summarizes structured fields compactly', () => {
		const rows = summarizeAgentBrain({
			memory: { type: 'auto', context_length: 20 } as any,
			output_schema: { type: 'object' } as any
		})
		expect(rows).toEqual([
			{ label: 'Memory', value: 'auto' },
			{ label: 'Output schema', value: 'configured' }
		])
	})
})

describe('inputTransformsToAgentConfig', () => {
	it('captures only static brain values and carries tools', () => {
		const config = inputTransformsToAgentConfig(
			{
				provider: { type: 'static', value: { kind: 'openai' } },
				system_prompt: { type: 'static', value: 'hi' },
				temperature: { type: 'javascript', expr: 'flow_input.t' }, // non-static → dropped
				max_iterations: { type: 'static', value: undefined }, // undefined → dropped
				user_message: { type: 'static', value: 'hello' } // not a brain key → dropped
			} as any,
			[{ id: 't1' }] as any
		)
		expect(config).toEqual({
			tools: [{ id: 't1' }],
			provider: { kind: 'openai' },
			system_prompt: 'hi'
		})
	})

	it('defaults tools to []', () => {
		expect(inputTransformsToAgentConfig({}, undefined)).toEqual({ tools: [] })
	})
})

describe('agentConfigToInputTransforms', () => {
	it('wraps brain values as static transforms and round-trips through the config', () => {
		const config = { provider: { kind: 'openai' }, system_prompt: 'hi', tools: [{ id: 't1' }] }
		const its = agentConfigToInputTransforms(config as any)
		expect(its).toEqual({
			provider: { type: 'static', value: { kind: 'openai' } },
			system_prompt: { type: 'static', value: 'hi' }
		})
		const back = inputTransformsToAgentConfig(its, config.tools as any)
		expect(back).toEqual(config)
	})
})

describe('nonStaticBrainKeys', () => {
	it('lists brain keys with a non-static transform, in brain-key order', () => {
		expect(
			nonStaticBrainKeys({
				provider: { type: 'static', value: {} },
				temperature: { type: 'connected' },
				system_prompt: { type: 'javascript', expr: 'x' }
			} as any)
		).toEqual(['system_prompt', 'temperature'])
	})
})

describe('flowLocalInputs', () => {
	it('keeps only user_message/user_attachments, dropping brain transforms', () => {
		expect(
			flowLocalInputs({
				provider: { type: 'static', value: {} },
				user_message: { type: 'static', value: 'hi' },
				user_attachments: { type: 'static', value: [] }
			} as any)
		).toEqual({
			user_message: { type: 'static', value: 'hi' },
			user_attachments: { type: 'static', value: [] }
		})
	})

	it('handles undefined', () => {
		expect(flowLocalInputs(undefined)).toEqual({})
	})
})
