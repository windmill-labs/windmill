import type { Schema, SchemaProperty } from '$lib/common'
import { keepsManagedMemory } from '$lib/components/flows/agentFormFields'

/**
 * The inputs of a raw app's agent runnable, after the agent's memory as its own page reads it: a
 * chat (`memory_id`, the conversation and memory key) when it keeps managed memory, single runs
 * answered from the history the app passes (`previous_messages`) otherwise. The server accepts
 * both either way (see `execute_component`), so a schema gone stale only mistypes the inputs.
 */
export function agentRunnableSchema(memory: unknown): Schema {
	const history: Record<string, SchemaProperty> = keepsManagedMemory(memory)
		? {
				memory_id: {
					type: 'string',
					description:
						'The conversation the message belongs to: reuse it to continue one, or omit it for a single run'
				}
			}
		: {
				previous_messages: {
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
				items: { type: 'object' }
			}
		},
		required: ['user_message'],
		order: ['user_message', ...Object.keys(history), 'user_attachments']
	}
}
