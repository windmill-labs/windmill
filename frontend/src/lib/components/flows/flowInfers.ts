import { inferArgs, loadSchemaFromPath } from '$lib/infer'
import { loadSchemaFlow } from '$lib/scripts'
import type { Schema } from '$lib/common'
import { emptySchema } from '$lib/utils'
import type { FlowModule, InputTransform } from '$lib/gen'
import { AGENT_FLOW_LOCAL_KEYS } from './agentResourceUtils'

export const AI_AGENT_SCHEMA: Schema = {
	$schema: 'https://json-schema.org/draft/2020-12/schema',
	properties: {
		provider: {
			type: 'object',
			format: 'ai-provider'
		},
		output_type: {
			type: 'string',
			description:
				'Whether the answer is text or an image. An image needs S3 storage on the workspace, and ignores tools.',
			enum: ['text', 'image'],
			default: 'text'
		},
		user_message: {
			type: 'string',
			description: 'The message sent to the agent as the user turn.'
		},
		system_prompt: {
			type: 'string',
			description: 'Sets how the agent behaves. Sent ahead of everything else.',
			// The one field people write paragraphs into, so it opens as a text area.
			minRows: 5,
			placeholder:
				"You are a support agent.\nLook up an answer with your tools before replying.\nCite what you used, and say you don't know rather than guessing."
		},
		streaming: {
			type: 'boolean',
			description: 'Stream the answer as it is produced.',
			default: true,
			showExpr: "fields.output_type !== 'image'"
		},
		memory: {
			type: 'object',
			description: 'History sent between the system message and the user message.',
			oneOf: [
				{
					type: 'object',
					title: 'off',
					properties: {
						kind: {
							type: 'string',
							enum: ['off'],
							description: 'Disable conversation memory'
						}
					}
				},
				{
					type: 'object',
					title: 'auto',
					properties: {
						kind: {
							type: 'string',
							enum: ['auto'],
							default: 'auto',
							description: 'Automatically manage conversation history'
						},
						context_length: {
							type: 'number',
							description:
								'Number of most recent messages to store and load. Set to 0 to disable memory.',
							default: 5
						},
						memory_id: {
							type: 'string',
							format: 'uuid',
							'x-auto-generate': true,
							description:
								'Custom memory identifier. Each unique ID maintains separate conversation history.',
							hideWhenChatEnabled: true
						}
					},
					required: ['kind'],
					'x-no-s3-storage-workspace-warning':
						'When no S3 storage is configured in your workspace settings, memory will be stored in database, which implies a limit of 100KB per memory entry. If you need to store more messages, you should use S3 storage in your workspace settings.'
				},
				{
					type: 'object',
					title: 'manual',
					properties: {
						kind: {
							type: 'string',
							enum: ['manual'],
							description:
								'Manually provide conversation messages, bypassing automatic memory management'
						},
						messages: {
							type: 'array',
							description: 'Array of conversation messages to use as history',
							items: {
								type: 'object',
								properties: {
									role: {
										type: 'string',
										enum: ['user', 'assistant', 'system']
									},
									content: {
										type: 'string'
									},
									tool_calls: {
										type: 'array',
										nullable: true,
										items: {
											type: 'object',
											properties: {
												id: { type: 'string' },
												type: { type: 'string' },
												function: {
													type: 'object',
													properties: {
														name: { type: 'string' },
														arguments: { type: 'string' }
													}
												}
											}
										}
									},
									tool_call_id: {
										type: 'string',
										nullable: true,
										description: 'The ID of the tool call this message is responding to'
									}
								},
								required: ['role']
							}
						}
					},
					required: ['kind', 'messages']
				}
			],
			showExpr: "fields.output_type !== 'image'"
		},
		output_schema: {
			type: 'object',
			description: 'A JSON schema the answer has to follow.',
			format: 'json-schema',
			showExpr: "fields.output_type !== 'image'"
		},
		user_attachments: {
			type: 'array',
			description: 'Images or PDFs sent with the message. Needs S3 storage on the workspace.',
			items: {
				type: 'object',
				resourceType: 's3object'
			}
		},
		max_completion_tokens: {
			type: 'number',
			description: 'The most tokens the answer may use.'
		},
		temperature: {
			type: 'number',
			description: 'How random the generation is, from 0 for deterministic up to 2.'
		},
		max_iterations: {
			type: 'number',
			description: 'How many times the agent may loop over calling the model and running tools.',
			default: 10
		}
	},
	// `output_type` defaults to text on the backend, so leaving it unset is valid: the form drops
	// the row rather than showing a field whose value a run would ignore.
	required: ['provider'],
	type: 'object',
	order: [
		'provider',
		'output_type',
		'user_message',
		'system_prompt',
		'streaming',
		'memory',
		'output_schema',
		'user_attachments',
		'max_completion_tokens',
		'temperature',
		'max_iterations'
	]
}

function migrateAiAgentInputTransforms(
	inputTransforms: Record<string, InputTransform>
): Record<string, InputTransform> {
	// Migrate user_images → user_attachments
	if ('user_images' in inputTransforms && !('user_attachments' in inputTransforms)) {
		inputTransforms.user_attachments = inputTransforms.user_images
		delete inputTransforms.user_images
	}

	// Check if this has the legacy format
	if ('messages_context_length' in inputTransforms && !('memory' in inputTransforms)) {
		const legacyValue = inputTransforms.messages_context_length
		if (legacyValue) {
			if (legacyValue?.type === 'static') {
				inputTransforms.memory = {
					type: 'static',
					value: {
						kind: 'auto',
						context_length: legacyValue.value ?? 0
					}
				}
			} else if (legacyValue.type === 'javascript') {
				// For dynamic expressions, wrap in the new format
				inputTransforms.memory = {
					type: 'javascript',
					expr: `{ kind: 'auto', context_length: ${legacyValue.expr} }`
				}
			}

			// Remove the legacy field
			delete inputTransforms.messages_context_length
		}
	}

	return inputTransforms
}

export async function loadSchemaFromModule(
	module: FlowModule,
	// The acting workspace when the flow editor runs in an AI session; else the nav workspace.
	workspace?: string
): Promise<{
	input_transforms: Record<string, InputTransform>
	schema: Schema
}> {
	const mod = module.value

	if (mod.type == 'rawscript' || mod.type === 'script' || mod.type === 'flow') {
		let schema: Schema
		if (mod.type === 'rawscript') {
			schema = emptySchema()
			await inferArgs(
				mod.language!,
				mod.content ?? '',
				schema,
				module.id === 'preprocessor' ? 'preprocessor' : undefined
			)
		} else if (mod.type == 'script' && mod.path && mod.path != '') {
			schema = await loadSchemaFromPath(mod.path!, mod.hash, workspace)
		} else if (mod.type == 'flow' && mod.path && mod.path != '') {
			schema = await loadSchemaFlow(mod.path!, workspace)
		} else {
			return {
				input_transforms: {},
				schema: emptySchema()
			}
		}

		const keys = Object.keys(schema?.properties ?? {})

		let input_transforms = mod.input_transforms ?? {}

		if (JSON.stringify(keys.sort()) !== JSON.stringify(Object.keys(input_transforms).sort())) {
			input_transforms = keys.reduce((accu, key) => {
				let nv =
					input_transforms[key] ??
					(module.id == 'failure' && ['message', 'name', 'step_id'].includes(key)
						? { type: 'javascript', expr: `error.${key}` }
						: {
								type: 'static',
								value: undefined
							})
				accu[key] = nv
				return accu
			}, {})
		}

		return {
			input_transforms: input_transforms,
			schema: schema ?? emptySchema()
		}
	} else if (mod.type === 'aiagent') {
		let input_transforms = migrateAiAgentInputTransforms(mod.input_transforms ?? {})
		// A linked step's brain lives in the resource, so only the flow-local inputs get a placeholder
		// transform: filling the brain keys back in would re-add the very fields linking strips, and
		// they would be persisted on the next save.
		const keys = mod.agent
			? (AGENT_FLOW_LOCAL_KEYS as readonly string[])
			: Object.keys(AI_AGENT_SCHEMA.properties ?? {})
		return {
			input_transforms: keys.reduce((accu, key) => {
				accu[key] = input_transforms[key] ?? {
					type: 'static',
					value: undefined
				}
				return accu
			}, {}),
			schema: AI_AGENT_SCHEMA
		}
	}

	return {
		input_transforms: {},
		schema: emptySchema()
	}
}
