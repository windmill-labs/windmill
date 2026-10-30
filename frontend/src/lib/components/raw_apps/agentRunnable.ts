import type { Schema, SchemaProperty } from '$lib/common'
import { agentMemoryMode } from '$lib/components/flows/agentFormFields'

/**
 * The inputs of a raw app's agent runnable. An agent whose memory is off answers from the history
 * the app passes (`previous_messages`); one keeping memory takes the conversation as
 * `backendAsync`'s `memoryId` option, which is not an input. The server accepts
 * `previous_messages` either way (see `execute_component`), so a schema gone stale only mistypes
 * the inputs.
 */
export function agentRunnableSchema(memory: unknown): Schema {
	const history: Record<string, SchemaProperty> = {}
	if (agentMemoryMode(memory) === 'off') {
		history.previous_messages = {
			type: 'array',
			description: 'The history to answer from, as chat messages: this agent keeps no memory',
			items: { type: 'object' }
		}
	}
	return {
		$schema: 'https://json-schema.org/draft/2020-12/schema',
		type: 'object',
		properties: {
			user_message: { type: 'string', description: 'The message to answer' },
			...history,
			user_attachments: {
				type: 'array',
				description: 'Files the message carries, as object storage references',
				items: { type: 'object', resourceType: 's3object' }
			}
		},
		required: ['user_message'],
		order: ['user_message', ...Object.keys(history), 'user_attachments']
	}
}
