import type { Schema } from '$lib/common'

/**
 * The inputs of a raw app's agent runnable, the same for every agent: the server runs one chat
 * turn from them and takes nothing else (see `execute_component`). `memory_id` names the
 * conversation, which is the agent's memory key, not an input of the run; without it the run is
 * a single one.
 */
export const AGENT_RUNNABLE_SCHEMA: Schema = {
	$schema: 'https://json-schema.org/draft/2020-12/schema',
	type: 'object',
	properties: {
		user_message: { type: 'string', description: 'The message to answer' },
		memory_id: {
			type: 'string',
			description:
				'The conversation the message belongs to: reuse it to continue one, or omit it for a single run'
		},
		user_attachments: {
			type: 'array',
			description: 'Files the message carries, as object storage references',
			items: { type: 'object' }
		}
	},
	required: ['user_message'],
	order: ['user_message', 'memory_id', 'user_attachments']
}
