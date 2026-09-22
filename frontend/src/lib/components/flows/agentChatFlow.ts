import type { FlowModule, InputTransform, OpenFlow } from '$lib/gen'
import { agentStreamingEnabled } from './agentFormFields'
import type { AgentTool } from './agentToolUtils'

/** The id of the one step an agent chat flow holds. */
export const AGENT_CHAT_STEP_ID = '__wm_agent_root'

/** The step's configuration as a flow step carries it: the brain as input transforms, plus tools. */
export interface AgentStepShape {
	input_transforms?: Record<string, InputTransform>
	tools?: AgentTool[]
}

/**
 * Whether the chat can run this agent. It keys the agent's history on the conversation, which
 * the worker only reads back for `auto` memory: any other setting would show a transcript the
 * model never sees.
 */
export function agentChatReady(step: AgentStepShape | undefined): boolean {
	const memory = step?.input_transforms?.memory as { value?: { kind?: string } } | undefined
	return memory?.value?.kind === 'auto'
}

/** `agentChatReady` over a saved agent's config, as a resource or draft holds it. */
export function agentArgsChatReady(args: Record<string, any> | undefined): boolean {
	return (args?.memory as { kind?: string } | undefined)?.kind === 'auto'
}

export const AGENT_CHAT_BLOCKED_REASON = 'Set memory to auto to chat with this agent.'

/** Whether a turn streams, from the same settings a flow's agent step reads. */
export function agentChatStreams(step: AgentStepShape | undefined): boolean {
	return agentStreamingEnabled(step as Record<string, any> | undefined)
}

/**
 * An agent as the one-step chat flow a turn previews. The step reads the message and the
 * attachments from the flow input, as a chat-enabled flow's agent step does, and the flow is
 * marked chat-enabled so the server files the run under a conversation.
 */
export function agentChatFlow(step: AgentStepShape | undefined): OpenFlow {
	const module: FlowModule = {
		id: AGENT_CHAT_STEP_ID,
		value: {
			type: 'aiagent',
			tools: Array.isArray(step?.tools) ? step.tools : [],
			input_transforms: {
				...(step?.input_transforms ?? {}),
				user_message: { type: 'javascript', expr: 'flow_input.user_message' },
				user_attachments: { type: 'javascript', expr: 'flow_input.user_attachments' }
			}
		} as any
	}
	return {
		summary: '',
		value: { modules: [module], chat_input_enabled: true } as OpenFlow['value'],
		schema: {
			$schema: 'https://json-schema.org/draft/2020-12/schema',
			type: 'object',
			properties: {
				user_message: {
					type: 'string',
					description: 'The message sent to the agent as the user turn.'
				},
				user_attachments: {
					type: 'array',
					description: 'Images or PDFs sent with the message. Needs S3 storage on the workspace.',
					items: { type: 'object', resourceType: 's3object' }
				}
			},
			required: ['user_message'],
			order: ['user_message', 'user_attachments']
		}
	}
}
