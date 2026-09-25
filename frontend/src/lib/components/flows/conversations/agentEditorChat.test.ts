import { describe, expect, it } from 'vitest'
import type { FlowModule } from '$lib/gen'
import { agentChatFlow, agentChatGap, agentChatPath } from './agentEditorChat'

describe('agentChatPath', () => {
	it('is a path no flow can take, so a same-path flow never shares its conversations', () => {
		const flowPath = /^[ufg](\/[\w-]+){2,}$/
		expect('f/support/agent').toMatch(flowPath)
		expect(agentChatPath('f/support/agent')).not.toMatch(flowPath)
	})
})

function agent(inputTransforms: Record<string, any>): FlowModule {
	return {
		id: '__wm_agent_root',
		value: { type: 'aiagent', tools: [{ id: 't' }], input_transforms: inputTransforms }
	} as unknown as FlowModule
}

describe('agentChatFlow', () => {
	it('runs the agent in chat mode on the message and files, never on a step memory id', () => {
		const flow = agentChatFlow(
			agent({
				system_prompt: { type: 'static', value: 'Be brief' },
				user_message: { type: 'static', value: 'a default' },
				memory_id: { type: 'static' },
				previous_messages: { type: 'static' }
			})
		)
		expect(flow.value.chat_input_enabled).toBe(true)
		const step = flow.value.modules[0].value as any
		expect(step.tools).toEqual([{ id: 't' }])
		expect(step.input_transforms).toEqual({
			system_prompt: { type: 'static', value: 'Be brief' },
			user_message: { type: 'javascript', expr: 'flow_input.user_message' },
			user_attachments: { type: 'javascript', expr: 'flow_input.user_attachments' }
		})
	})
})

describe('agentChatGap', () => {
	it('is nothing to fix with managed memory and streaming left at its default', () => {
		expect(
			agentChatGap({ memory: { type: 'static', value: { kind: 'window', context_length: 10 } } })
		).toBeUndefined()
	})

	it('reads an agent with no memory setting as off, and offers to turn it on', () => {
		expect(agentChatGap({})).toEqual({ memory: true, memoryCanTurnOn: true, noStream: undefined })
	})

	it('flags memory off and streaming off', () => {
		expect(
			agentChatGap({
				memory: { type: 'static', value: { kind: 'off' } },
				streaming: { type: 'static', value: false }
			})
		).toEqual({ memory: true, memoryCanTurnOn: true, noStream: 'off' })
	})

	it('does not offer to replace a legacy manual memory', () => {
		expect(
			agentChatGap({ memory: { type: 'static', value: { kind: 'manual', messages: [] } } })
		).toEqual({ memory: true, memoryCanTurnOn: false, noStream: undefined })
	})

	it('flags an image answer as never streaming, whatever the setting', () => {
		expect(
			agentChatGap({
				memory: { type: 'static', value: { kind: 'window', context_length: 10 } },
				output_type: { type: 'static', value: 'image' },
				streaming: { type: 'static', value: true }
			})
		).toEqual({ memory: false, memoryCanTurnOn: false, noStream: 'image' })
	})
})
