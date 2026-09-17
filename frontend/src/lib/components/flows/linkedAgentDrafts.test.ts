import { describe, expect, it } from 'vitest'
import { inlineAgentDraft } from './linkedAgentDrafts'

describe('inlineAgentDraft', () => {
	// The worker never reads a history input from the resource, so a preview of the draft must not
	// either: a draft carrying one would test against a memory the deployed step never sees.
	it('keeps the brain and the step inputs, never a history input the draft carries', () => {
		const step = {
			type: 'aiagent',
			agent: 'u/admin/agent',
			tools: [],
			input_transforms: {
				user_message: { type: 'javascript', expr: 'flow_input.user_message' },
				memory_id: { type: 'static', value: 'cust-1' }
			}
		} as any
		const draft = {
			provider: { kind: 'openai', resource: '$res:u/admin/openai', model: 'm' },
			memory: { kind: 'window', context_length: 10 },
			memory_id: 'from-the-draft',
			previous_messages: [{ role: 'user', content: 'from the draft' }]
		} as any
		const inlined = inlineAgentDraft(step, draft)
		expect(inlined.input_transforms.memory).toEqual({
			type: 'static',
			value: { kind: 'window', context_length: 10 }
		})
		expect(inlined.input_transforms.memory_id).toEqual({ type: 'static', value: 'cust-1' })
		expect(inlined.input_transforms.previous_messages).toBeUndefined()
	})
})
