import type { AiAgent, FlowModule, InputTransform, OpenFlow } from '$lib/gen'
import { AGENT_FLOW_LOCAL_KEYS } from '../agentResourceUtils'
import { agentMemoryMode, keepsManagedMemory } from '../agentFormFields'

/**
 * The inputs of the flow a saved agent is chatted with: the message, and the files the composer's
 * paperclip uploads. `user_message` is the name the server requires of a chat-mode run.
 */
export const AGENT_CHAT_SCHEMA = {
	$schema: 'https://json-schema.org/draft/2020-12/schema',
	type: 'object',
	properties: {
		user_message: { type: 'string', description: 'Message from user' },
		user_attachments: {
			type: 'array',
			items: { type: 'object', resourceType: 's3object' },
			description: 'Images or PDFs for the agent to read'
		}
	},
	required: ['user_message']
}

/**
 * The chat-mode flow one turn of the agent editor's chat runs: the agent as edited, alone, reading
 * the message and files from the chat. Every other flow-local input is dropped, `memory_id` above
 * all: a step memory id, even an empty one, would replace the conversation's and the agent would
 * not remember the turns before.
 */
export function agentChatFlow(agent: FlowModule): OpenFlow {
	const value = agent.value as AiAgent
	const brain = Object.fromEntries(
		Object.entries(value.input_transforms ?? {}).filter(
			([key]) => !(AGENT_FLOW_LOCAL_KEYS as readonly string[]).includes(key)
		)
	)
	return {
		summary: '',
		value: {
			chat_input_enabled: true,
			modules: [
				{
					id: agent.id,
					value: {
						...value,
						input_transforms: {
							...brain,
							user_message: { type: 'javascript', expr: 'flow_input.user_message' },
							user_attachments: { type: 'javascript', expr: 'flow_input.user_attachments' }
						} as AiAgent['input_transforms']
					}
				}
			]
		},
		schema: AGENT_CHAT_SCHEMA
	}
}

/** What the agent as configured takes away from a chat. */
export type AgentChatGap = {
	/** No managed memory: every message would be answered without the ones before it, so the
	 *  chat is not offered at all. */
	memory: boolean
	/** Whether turning memory on is a plain switch, rather than replacing an older manual list. */
	memoryCanTurnOn: boolean
	/** Why a turn shows nothing until its run ends: streaming switched off, or an image answer,
	 *  which the worker never streams whatever the setting. */
	noStream: 'off' | 'image' | undefined
}

function staticValue(transform: InputTransform | undefined): unknown {
	return transform?.type === 'static' ? transform.value : undefined
}

export function agentChatGap(
	transforms: Record<string, InputTransform> | undefined
): AgentChatGap | undefined {
	const memory = staticValue(transforms?.['memory'])
	const gap: AgentChatGap = {
		memory: !keepsManagedMemory(memory),
		memoryCanTurnOn: agentMemoryMode(memory) === 'off',
		noStream:
			staticValue(transforms?.['output_type']) === 'image'
				? 'image'
				: // Unset streams: it is on by default.
					staticValue(transforms?.['streaming']) === false
					? 'off'
					: undefined
	}
	return gap.memory || gap.noStream ? gap : undefined
}
