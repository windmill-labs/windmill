import type { VariantConfig } from '../evalVariants'
import type { Tool } from '../../../../shared'
import type { FlowAIChatHelpers } from '../../../core'

/**
 * A single tool that sets the entire flow JSON at once.
 */
const setFlowJsonTool: Tool<FlowAIChatHelpers> = {
	def: {
		type: 'function',
		function: {
			name: 'set_flow_json',
			description: 'Set the entire flow by providing the modules array. This replaces all existing modules.',
			parameters: {
				type: 'object',
				properties: {
					modules: {
						type: 'array',
						description: 'Array of flow modules',
						items: {
							type: 'object',
							properties: {
								id: { type: 'string', description: 'Unique module ID (e.g., "a", "b", "step_1")' },
								value: {
									type: 'object',
									description: 'Module value - varies by type',
									properties: {
										type: {
											type: 'string',
											enum: ['rawscript', 'script', 'flow', 'forloopflow', 'branchone', 'branchall'],
											description: 'Module type'
										},
										content: { type: 'string', description: 'For rawscript: the code content' },
										language: {
											type: 'string',
											enum: ['bun', 'python3', 'deno', 'bash', 'go', 'postgresql'],
											description: 'For rawscript: programming language'
										},
										input_transforms: {
											type: 'object',
											description: 'Input parameter mappings',
											additionalProperties: {
												type: 'object',
												properties: {
													type: { type: 'string', enum: ['static', 'javascript'] },
													value: { type: 'string' },
													expr: { type: 'string' }
												}
											}
										}
									},
									required: ['type']
								}
							},
							required: ['id', 'value']
						}
					}
				},
				required: ['modules']
			}
		}
	},
	fn: async ({ args, helpers }) => {
		const { modules } = args
		await helpers.setFlowJson(JSON.stringify({ modules }))
		return `Flow updated with ${modules.length} module(s): [${modules.map((m: any) => m.id).join(', ')}]`
	}
}

/**
 * Minimal system prompt - much shorter than production.
 */
const MINIMAL_SYSTEM_PROMPT = `You are a workflow assistant that creates Windmill flows.

A flow is a sequence of modules. Each module has an "id" and a "value".

## Module Types

1. **rawscript** - Inline code
   - language: "bun" (TypeScript), "python3", "bash", etc.
   - content: The code as a string
   - input_transforms: Maps inputs to the script parameters

2. **script** - Reference to existing script (path: "f/folder/script_name")

3. **forloopflow** - Loop over an iterator
4. **branchone** / **branchall** - Conditional branching

## Example: Simple TypeScript step

{
  "modules": [{
    "id": "a",
    "value": {
      "type": "rawscript",
      "language": "bun",
      "content": "export async function main() {\\n  return \\"Hello World\\";\\n}",
      "input_transforms": {}
    }
  }]
}

## Accessing previous results

Use \`results.step_id\` in input_transforms with type "javascript":
"input_transforms": {
  "param_name": { "type": "javascript", "expr": "results.a" }
}

Use the set_flow_json tool to create or modify the flow.`

/**
 * Minimal single-tool variant.
 * Uses a concise system prompt and only the set_flow_json tool.
 */
export const MINIMAL_SINGLE_TOOL_VARIANT: VariantConfig = {
	name: 'minimal-single-tool',
	description: 'Minimal prompt with single set_flow_json tool',
	systemPrompt: {
		type: 'custom',
		content: MINIMAL_SYSTEM_PROMPT
	},
	tools: {
		type: 'custom',
		tools: [setFlowJsonTool]
	}
}
