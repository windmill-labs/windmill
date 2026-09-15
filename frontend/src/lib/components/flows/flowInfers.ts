import { inferArgs, loadSchemaFromPath } from '$lib/infer'
import { loadSchemaFlow } from '$lib/scripts'
import type { Schema } from '$lib/common'
import { emptySchema } from '$lib/utils'
import type { FlowModule, InputTransform } from '$lib/gen'
import { AGENT_FLOW_LOCAL_KEYS } from './agentResourceUtils'
import { AGENT_HISTORY_KEYS } from './agentFormFields'

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
			description:
				'Windmill stores the conversation and sends its last messages with each request.',
			enumLabels: {
				off: 'Off',
				window: 'On',
				auto: 'On (legacy)',
				manual: 'Previous messages (legacy)'
			},
			oneOf: [
				{
					type: 'object',
					title: 'off',
					properties: {
						kind: { type: 'string', enum: ['off'] }
					}
				},
				{
					type: 'object',
					title: 'window',
					properties: {
						kind: { type: 'string', enum: ['window'] },
						context_length: {
							type: 'number',
							title: 'Messages to keep',
							description: 'Number of most recent messages to load and store.',
							default: 10
						}
					},
					required: ['kind', 'context_length']
				}
			],
			showExpr: "fields.output_type !== 'image'"
		},
		memory_id: {
			type: 'string',
			description:
				'Names the memory this step reads and writes, overriding the memory id the run was started with. Read only while managed memory is on.',
			showExpr: "fields.output_type !== 'image'"
		},
		messages: {
			type: 'array',
			description:
				'History the flow supplies, sent before the user message. Read only while managed memory is off.',
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
			},
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
		// The step's own roster fills `items.enum` in, so the static editor offers the tools this
		// agent actually has (`AiAgentStepInputs`). Absence, not an empty list, is what carries every
		// tool: a step that holds the field and names nothing has chosen to advertise none.
		// Shown for image output as the roster it narrows is, even though neither is used there.
		enabled_tools: {
			type: 'array',
			// Deliberately short. It is the only place the field's text is always on screen rather than
			// behind the row's tooltip, and the surface it shows on is the run form, which offers the
			// names in a picker and has no unset state to explain.
			description: 'Which of the agent tools a run may call.',
			items: {
				type: 'string'
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
		'memory_id',
		'messages',
		'output_schema',
		'user_attachments',
		'enabled_tools',
		'max_completion_tokens',
		'temperature',
		'max_iterations'
	]
}

/** Memory shapes older editors wrote. The step form offers one only to a step that still holds it,
 *  since the one-of field rewrites a value that matches none of its options. */
export const LEGACY_MEMORY_VARIANTS: Record<string, any> = {
	auto: {
		type: 'object',
		title: 'auto',
		properties: {
			kind: { type: 'string', enum: ['auto'] },
			context_length: { type: 'number', title: 'Messages to keep', default: 10 },
			memory_id: { type: 'string', title: 'Fixed memory id' }
		},
		required: ['kind']
	},
	manual: {
		type: 'object',
		title: 'manual',
		properties: {
			kind: { type: 'string', enum: ['manual'] },
			messages: { type: 'array', items: AI_AGENT_SCHEMA.properties?.messages?.items }
		},
		required: ['kind', 'messages']
	}
}

/** The memory property to render for a value: a legacy kind is added as an option only while the
 *  value holds it. Otherwise the property itself is returned, which callers compare by identity to
 *  avoid rebuilding the step schema. */
export function memoryPropertyFor(property: any, value: any): any {
	let legacy = value?.kind ? LEGACY_MEMORY_VARIANTS[value.kind] : undefined
	if (!legacy || !property?.oneOf) return property
	// The form fills an empty string field with `''` when it opens, so the baked id field is only
	// offered to a value saved with the key. Keyed on presence rather than content, or clearing the
	// id to retype it would remove the field mid-edit.
	if (value.kind === 'auto' && !('memory_id' in value)) {
		const { memory_id: _, ...properties } = legacy.properties
		legacy = { ...legacy, properties }
	}
	return { ...property, oneOf: [...property.oneOf, legacy] }
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
				const transform =
					input_transforms[key] ??
					// A present history input is the step's choice at runtime, so it gets no placeholder.
					((AGENT_HISTORY_KEYS as readonly string[]).includes(key)
						? undefined
						: { type: 'static', value: undefined })
				if (transform) accu[key] = transform
				return accu
			}, {}),
			// A copy per step, never the shared constant: the form writes back into the property it
			// renders (`InputTransformForm` binds `schema.properties[argName]`), and the tool names
			// one step offers would otherwise become every step's.
			schema: structuredClone(AI_AGENT_SCHEMA)
		}
	}

	return {
		input_transforms: {},
		schema: emptySchema()
	}
}
