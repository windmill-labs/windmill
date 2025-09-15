import { inferArgs, loadSchemaFromPath } from '$lib/infer'
import { loadSchemaFlow } from '$lib/scripts'
import type { Schema } from '$lib/common'
import { emptySchema } from '$lib/utils'
import type { FlowModule, InputTransform } from '$lib/gen'

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
		const schema = {
			$schema: 'https://json-schema.org/draft/2020-12/schema',
			properties: {
				provider: {
					type: 'object',
					format: 'ai-provider'
				},
				user_message: {
					type: 'string'
				},
				system_prompt: {
					type: 'string'
				},
				images: {
					type: 'array',
					items: {
						type: 'object',
						properties: {
							s3_object: {
								type: 'object',
								format: 'resource-s3_object'
							}
						}
					}
				},
				max_completion_tokens: {
					type: 'number'
				},
				temperature: {
					type: 'number',
					description:
						'Controls randomness in text generation. Range: 0.0 (deterministic) to 2.0 (random).'
				},
				output_type: {
					type: 'string',
					description:
						'The type of output the AI agent will generate (text or image). Image output will ignore tools, and only works with OpenAI, Google AI and OpenRouter gemini-image-preview model.',
					enum: ['text', 'image'],
					default: 'text'
				},
				output_schema: {
					type: 'object',
					description:
						'JSON schema that the AI agent will follow for its response format (only used if output_type is text)',
					format: 'json-schema'
				}
			},
			required: ['provider', 'model', 'user_message'],
			type: 'object',
			order: [
				'provider',
				'model',
				'user_message',
				'system_prompt',
				'images',
				'max_completion_tokens',
				'temperature',
				'output_type',
				'output_schema'
			]
		}
		let input_transforms = mod.input_transforms ?? {}
		return {
			input_transforms: Object.keys(schema?.properties ?? {}).reduce((accu, key) => {
				accu[key] = input_transforms[key] ?? {
					type: 'static',
					value: undefined
				}
				return accu
			}, {}),
			schema
		}
	}

	return {
		input_transforms: {},
		schema: emptySchema()
	}
}
