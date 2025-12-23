import { inferArgs, loadSchemaFromPath } from '$lib/infer'
import { loadSchemaFlow } from '$lib/scripts'
import type { Schema } from '$lib/common'
import { emptySchema } from '$lib/utils'
import type { FlowModule, InputTransform } from '$lib/gen'

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
				'The type of output the AI agent will generate (text or image). Image output requires a configured workspace S3 storage, will ignore tools, and only works with OpenAI, Google AI and OpenRouter gemini-image-preview model.',
			enum: ['text', 'image'],
			default: 'text'
		},
		user_message: {
			type: 'string',
			description:
				'The message to give as input to the AI agent. Optional when messages array is provided. You can turn on chat input mode on the input interface to link this field to the message sent by the user.'
		},
		system_prompt: {
			type: 'string',
			description: 'The system prompt to give as input to the AI agent.'
		},
		streaming: {
			type: 'boolean',
			description: 'Whether to stream the output of the AI agent.',
			default: true,
			showExpr: "fields.output_type === 'text'"
		},
		memory: {
			type: 'object',
			description:
				'Configure how conversation memory is managed. Choose "auto" to let Windmill automatically store and load messages (up to N last messages), or "manual" to provide an explicit array of conversation messages. The system_prompt and user_message are added to the messages if provided.',
			oneOf: [
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
							default: 0
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
			showExpr: "fields.output_type === 'text'"
		},
		output_schema: {
			type: 'object',
			description: 'JSON schema that the AI agent will follow for its response format.',
			format: 'json-schema',
			showExpr: "fields.output_type === 'text'"
		},
		user_images: {
			type: 'array',
			description:
				'Array of images to give as input to the AI agent. Requires a configured workspace S3 storage.',
			items: {
				type: 'object',
				resourceType: 's3object'
			}
		},
		max_completion_tokens: {
			type: 'number',
			description: 'The maximum number of output tokens.'
		},
		temperature: {
			type: 'number',
			description:
				'Controls randomness in text generation. Range: 0.0 (deterministic) to 2.0 (random).',
			showExpr: "fields.output_type === 'text'"
		},
		max_iterations: {
			type: 'number',
			description:
				'Limits how many times the agent can loop through reasoning and tool use. Range: 1-1000.',
			default: 10
		}
	},
	required: ['provider', 'output_type'],
	type: 'object',
	order: [
		'provider',
		'output_type',
		'user_message',
		'system_prompt',
		'streaming',
		'memory',
		'output_schema',
		'user_images',
		'max_completion_tokens',
		'temperature',
		'max_iterations'
	]
}

function migrateAiAgentInputTransforms(
	inputTransforms: Record<string, InputTransform>
): Record<string, InputTransform> {
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

export async function loadSchemaFromModule(module: FlowModule): Promise<{
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
			schema = await loadSchemaFromPath(mod.path!, mod.hash)
		} else if (mod.type == 'flow' && mod.path && mod.path != '') {
			schema = await loadSchemaFlow(mod.path!)
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
		return {
			input_transforms: Object.keys(AI_AGENT_SCHEMA.properties ?? {}).reduce((accu, key) => {
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
