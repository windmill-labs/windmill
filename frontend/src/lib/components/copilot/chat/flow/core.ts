import {
	ScriptService,
	type FlowModule,
	type InputTransform,
	type RawScript,
	type Script,
	JobService
} from '$lib/gen'
import type {
	ChatCompletionSystemMessageParam,
	ChatCompletionUserMessageParam
} from 'openai/resources/chat/completions.mjs'
import type { ChatCompletionTool as ChatCompletionFunctionTool } from 'openai/resources/chat/completions.mjs'
import { z } from 'zod'
import uFuzzy from '@leeoniya/ufuzzy'
import { emptyString } from '$lib/utils'
import {
	createDbSchemaTool,
	getFormattedResourceTypes,
	getLangContext,
	SUPPORTED_CHAT_SCRIPT_LANGUAGES
} from '../script/core'
import {
	createSearchHubScriptsTool,
	createToolDef,
	type Tool,
	executeTestRun,
	buildSchemaForTool,
	buildTestRunArgs,
	buildContextString,
	applyCodePiecesToFlowModules,
	findModuleById,
	SPECIAL_MODULE_IDS
} from '../shared'
import type { ContextElement } from '../context'
import type { ExtendedOpenFlow } from '$lib/components/flows/types'
import openFlowSchema from './openFlow.json'
import { inlineScriptStore, extractAndReplaceInlineScripts } from './inlineScriptsUtils'

/**
 * Helper interface for AI chat flow operations
 *
 * Note: AI chat is only responsible for setting the beforeFlow snapshot when making changes.
 * Accept/reject operations are exposed here but implemented via flowDiffManager.
 */
export interface FlowAIChatHelpers {
	// flow context
	getFlowAndSelectedId: () => { flow: ExtendedOpenFlow; selectedId: string }
	getModules: (id?: string) => FlowModule[]

	// snapshot management (AI sets this when making changes)
	/** Set the before flow snapshot */
	setSnapshot: (snapshot: ExtendedOpenFlow) => void
	/** Revert the entire flow to a previous snapshot */
	revertToSnapshot: (snapshot?: ExtendedOpenFlow) => void

	// ai chat tools
	setCode: (id: string, code: string) => Promise<void>
	setFlowJson: (
		modules: FlowModule[] | undefined,
		schema: Record<string, any> | undefined
	) => Promise<void>
	getFlowInputsSchema: () => Promise<Record<string, any>>
	/** Update exprsToSet store for InputTransformForm components (only if module is selected) */
	updateExprsToSet: (id: string, inputTransforms: Record<string, InputTransform>) => void

	// accept/reject operations (via flowDiffManager)
	/** Accept all pending module changes */
	acceptAllModuleActions: () => void
	/** Reject all pending module changes */
	rejectAllModuleActions: () => void
	/** Check if there are pending changes requiring user approval */
	hasPendingChanges: () => boolean
	/** Select a step in the flow */
	selectStep: (id: string) => void

	/** Run a test of the current flow using the UI's preview mechanism */
	testFlow: (args?: Record<string, any>, conversationId?: string) => Promise<string | undefined>
}

const searchScriptsSchema = z.object({
	query: z
		.string()
		.describe('The query to search for, e.g. send email, list stripe invoices, etc..')
})

const searchScriptsToolDef = createToolDef(
	searchScriptsSchema,
	'search_scripts',
	'Search for scripts in the workspace. Returns array of {path, summary} objects.'
)

const langSchema = z.enum(
	SUPPORTED_CHAT_SCRIPT_LANGUAGES as [RawScript['language'], ...RawScript['language'][]]
)

const resourceTypeToolSchema = z.object({
	query: z.string().describe('The query to search for, e.g. stripe, google, etc..'),
	language: langSchema.describe(
		'The programming language the code using the resource type will be written in'
	)
})

const resourceTypeToolDef = createToolDef(
	resourceTypeToolSchema,
	'resource_type',
	'Search for resource types (e.g., postgresql, stripe). Returns formatted resource type definitions with usage examples.'
)

const getInstructionsForCodeGenerationToolSchema = z.object({
	language: langSchema.describe('The programming language the code will be written in')
})

const getInstructionsForCodeGenerationToolDef = createToolDef(
	getInstructionsForCodeGenerationToolSchema,
	'get_instructions_for_code_generation',
	'Get instructions for code generation for a raw script step'
)

const setFlowJsonToolDef: ChatCompletionFunctionTool = {
	type: 'function',
	function: {
		name: 'set_flow_json',
		description:
			'Set the entire flow by providing the complete flow object. This replaces all existing modules and schema.',
		strict: false,
		parameters: {
			type: 'object',
			properties: {
				modules: {
					type: 'string',
					description: 'JSON string containing the flow modules'
				},
				schema: {
					type: 'string',
					description: 'JSON string containing the flow input schema'
				}
			},
			required: []
		}
	}
}

class WorkspaceScriptsSearch {
	private uf: uFuzzy
	private workspace: string | undefined = undefined
	private scripts: Script[] | undefined = undefined

	constructor() {
		this.uf = new uFuzzy()
	}

	private async init(workspace: string) {
		this.scripts = await ScriptService.listScripts({
			workspace
		})
		this.workspace = workspace
	}

	async search(query: string, workspace: string) {
		if (this.scripts === undefined || this.workspace !== workspace) {
			await this.init(workspace)
		}

		const scripts = this.scripts

		if (!scripts) {
			throw new Error('Failed to load scripts')
		}

		const results = this.uf.search(
			scripts.map((s) => (emptyString(s.summary) ? s.path : s.summary + ' (' + s.path + ')')),
			query.trim()
		)
		const scriptResults =
			results[2]?.map((id) => ({
				path: scripts[id].path,
				summary: scripts[id].summary
			})) ?? []

		return scriptResults
	}
}

// Will be overridden by setSchema
const testRunFlowSchema = z.object({
	args: z
		.object({})
		.describe('Arguments to pass to the flow (optional, uses default flow inputs if not provided)')
})

const testRunFlowToolDef = createToolDef(
	testRunFlowSchema,
	'test_run_flow',
	'Execute a test run of the current flow'
)

const testRunStepSchema = z.object({
	stepId: z.string().describe('The id of the step to test'),
	args: z
		.object({})
		.nullable()
		.optional()
		.describe('Arguments to pass to the step (optional, uses default step inputs if not provided)')
})

const testRunStepToolDef = createToolDef(
	testRunStepSchema,
	'test_run_step',
	'Execute a test run of a specific step in the flow'
)

const inspectInlineScriptSchema = z.object({
	moduleId: z
		.string()
		.describe('The ID of the module whose inline script content you want to inspect')
})

const inspectInlineScriptToolDef = createToolDef(
	inspectInlineScriptSchema,
	'inspect_inline_script',
	'Inspect the full content of an inline script. Use this to view the actual script code before making changes with set_module_code.'
)

const setModuleCodeSchema = z.object({
	moduleId: z.string().describe('The ID of the module to set code for'),
	code: z.string().describe('The full script code content')
})

const setModuleCodeToolDef = createToolDef(
	setModuleCodeSchema,
	'set_module_code',
	'Set or modify the code for an existing inline script module. Use this for quick code-only changes. The module must already exist in the flow.'
)

const workspaceScriptsSearch = new WorkspaceScriptsSearch()

export const flowTools: Tool<FlowAIChatHelpers>[] = [
	createSearchHubScriptsTool(false),
	createDbSchemaTool<FlowAIChatHelpers>(),
	{
		def: searchScriptsToolDef,
		fn: async ({ args, workspace, toolId, toolCallbacks }) => {
			toolCallbacks.setToolStatus(toolId, {
				content: 'Searching for workspace scripts related to "' + args.query + '"...'
			})
			const parsedArgs = searchScriptsSchema.parse(args)
			const scriptResults = await workspaceScriptsSearch.search(parsedArgs.query, workspace)
			toolCallbacks.setToolStatus(toolId, {
				content:
					'Found ' +
					scriptResults.length +
					' scripts in the workspace related to "' +
					args.query +
					'"'
			})
			return JSON.stringify(scriptResults)
		}
	},
	{
		def: resourceTypeToolDef,
		fn: async ({ args, toolId, workspace, toolCallbacks }) => {
			const parsedArgs = resourceTypeToolSchema.parse(args)
			toolCallbacks.setToolStatus(toolId, {
				content: 'Searching resource types for "' + parsedArgs.query + '"...'
			})
			const formattedResourceTypes = await getFormattedResourceTypes(
				parsedArgs.language,
				parsedArgs.query,
				workspace
			)
			toolCallbacks.setToolStatus(toolId, {
				content: 'Retrieved resource types for "' + parsedArgs.query + '"'
			})
			return formattedResourceTypes
		}
	},
	{
		def: getInstructionsForCodeGenerationToolDef,
		fn: async ({ args, toolId, toolCallbacks }) => {
			const parsedArgs = getInstructionsForCodeGenerationToolSchema.parse(args)
			const langContext = getLangContext(parsedArgs.language, {
				allowResourcesFetch: true
			})
			toolCallbacks.setToolStatus(toolId, {
				content: 'Retrieved instructions for code generation in ' + parsedArgs.language
			})
			return langContext
		}
	},
	{
		def: testRunFlowToolDef,
		fn: async function ({ args, workspace, helpers, toolCallbacks, toolId }) {
			const { flow } = helpers.getFlowAndSelectedId()

			if (!flow || !flow.value) {
				toolCallbacks.setToolStatus(toolId, {
					content: 'No flow available to test',
					error: 'No flow found in current context'
				})
				throw new Error(
					'No flow available to test. Please ensure you have a flow open in the editor.'
				)
			}

			const parsedArgs = await buildTestRunArgs(args, this.def)
			// Use the UI test mechanism - this opens the preview panel
			return executeTestRun({
				jobStarter: async () => {
					const jobId = await helpers.testFlow(parsedArgs)
					if (!jobId) {
						throw new Error('Failed to start test run - testFlow returned undefined')
					}
					return jobId
				},
				workspace,
				toolCallbacks,
				toolId,
				startMessage: 'Starting flow test run...',
				contextName: 'flow'
			})
		},
		setSchema: async function (helpers: FlowAIChatHelpers) {
			await buildSchemaForTool(this.def, async () => {
				const flowInputsSchema = await helpers.getFlowInputsSchema()
				return flowInputsSchema
			})
		},
		requiresConfirmation: true,
		confirmationMessage: 'Run flow test',
		showDetails: true
	},
	{
		// set strict to false to avoid issues with open ai models
		def: { ...testRunStepToolDef, function: { ...testRunStepToolDef.function, strict: false } },
		fn: async ({ args, workspace, helpers, toolCallbacks, toolId }) => {
			const { flow } = helpers.getFlowAndSelectedId()

			if (!flow || !flow.value) {
				toolCallbacks.setToolStatus(toolId, {
					content: 'No flow available to test step from',
					error: 'No flow found in current context'
				})
				throw new Error(
					'No flow available to test step from. Please ensure you have a flow open in the editor.'
				)
			}

			const stepId = args.stepId
			const stepArgs = args.args || {}

			// Find the step in the flow
			const modules = helpers.getModules()
			let targetModule: FlowModule | undefined = findModuleById(modules, stepId)

			if (!targetModule) {
				toolCallbacks.setToolStatus(toolId, {
					content: `Step '${stepId}' not found in flow`,
					error: `Step with id '${stepId}' does not exist in the current flow`
				})
				throw new Error(
					`Step with id '${stepId}' not found in flow. Available steps: ${modules.map((m) => m.id).join(', ')}`
				)
			}

			const module = targetModule
			const moduleValue = module.value

			if (moduleValue.type === 'rawscript') {
				// Test raw script step

				return executeTestRun({
					jobStarter: () =>
						JobService.runScriptPreview({
							workspace: workspace,
							requestBody: {
								content: moduleValue.content ?? '',
								language: moduleValue.language,
								args:
									module.id === SPECIAL_MODULE_IDS.PREPROCESSOR
										? { _ENTRYPOINT_OVERRIDE: 'preprocessor', ...stepArgs }
										: stepArgs
							}
						}),
					workspace,
					toolCallbacks,
					toolId,
					startMessage: `Starting test run of step '${stepId}'...`,
					contextName: 'script'
				})
			} else if (moduleValue.type === 'script') {
				// Test script step - need to get the script content
				const script = moduleValue.hash
					? await ScriptService.getScriptByHash({
							workspace: workspace,
							hash: moduleValue.hash
						})
					: await ScriptService.getScriptByPath({
							workspace: workspace,
							path: moduleValue.path
						})

				return executeTestRun({
					jobStarter: () =>
						JobService.runScriptPreview({
							workspace: workspace,
							requestBody: {
								content: script.content,
								language: script.language,
								args:
									module.id === SPECIAL_MODULE_IDS.PREPROCESSOR
										? { _ENTRYPOINT_OVERRIDE: 'preprocessor', ...stepArgs }
										: stepArgs
							}
						}),
					workspace,
					toolCallbacks,
					toolId,
					startMessage: `Starting test run of script step '${stepId}'...`,
					contextName: 'script'
				})
			} else if (moduleValue.type === 'flow') {
				// Test flow step
				return executeTestRun({
					jobStarter: () =>
						JobService.runFlowByPath({
							workspace: workspace,
							path: moduleValue.path,
							requestBody: stepArgs
						}),
					workspace,
					toolCallbacks,
					toolId,
					startMessage: `Starting test run of flow step '${stepId}'...`,
					contextName: 'flow'
				})
			} else {
				toolCallbacks.setToolStatus(toolId, {
					content: `Step type '${moduleValue.type}' not supported for testing`,
					error: `Cannot test step of type '${moduleValue.type}'`
				})
				throw new Error(
					`Cannot test step of type '${moduleValue.type}'. Supported types: rawscript, script, flow`
				)
			}
		},
		requiresConfirmation: true,
		confirmationMessage: 'Run flow step test',
		showDetails: true
	},
	{
		def: inspectInlineScriptToolDef,
		fn: async ({ args, toolCallbacks, toolId }) => {
			const parsedArgs = inspectInlineScriptSchema.parse(args)
			const moduleId = parsedArgs.moduleId

			toolCallbacks.setToolStatus(toolId, {
				content: `Retrieving inline script content for module '${moduleId}'...`
			})

			const content = inlineScriptStore.get(moduleId)

			if (content === undefined) {
				toolCallbacks.setToolStatus(toolId, {
					content: `Module '${moduleId}' not found in inline script store`,
					error: `No inline script found for module ID '${moduleId}'`
				})
				throw new Error(
					`Module '${moduleId}' not found. This module either doesn't exist, isn't a rawscript, or wasn't replaced with a reference.`
				)
			}

			toolCallbacks.setToolStatus(toolId, {
				content: `Retrieved inline script for module '${moduleId}'`
			})

			return JSON.stringify({
				moduleId,
				content,
				note: 'To modify this script, use the set_module_code tool with the new code'
			})
		}
	},
	{
		def: setModuleCodeToolDef,
		streamArguments: true,
		showDetails: true,
		showFade: true,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const parsedArgs = setModuleCodeSchema.parse(args)
			const { moduleId, code } = parsedArgs

			toolCallbacks.setToolStatus(toolId, { content: `Setting code for module '${moduleId}'...` })

			// Update store to keep it coherent (for subsequent set_flow_json calls with references)
			inlineScriptStore.set(moduleId, code)

			// Update the flow directly via helper
			await helpers.setCode(moduleId, code)

			toolCallbacks.setToolStatus(toolId, {
				content: `Code updated for module '${moduleId}'`,
				result: 'Success'
			})
			return `Code for module '${moduleId}' has been updated successfully.`
		}
	},
	{
		def: setFlowJsonToolDef,
		streamArguments: true,
		showDetails: true,
		showFade: true,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const { modules, schema } = args
			const parsedModules = modules
				? typeof modules === 'string'
					? JSON.parse(modules)
					: modules
				: undefined
			const parsedSchema = schema
				? typeof schema === 'string'
					? JSON.parse(schema)
					: schema
				: undefined
			toolCallbacks.setToolStatus(toolId, {
				content: `Setting flow...`
			})
			await helpers.setFlowJson(parsedModules, parsedSchema)
			toolCallbacks.setToolStatus(toolId, {
				content: `Flow updated`,
				result: 'Success'
			})
			return `Flow updated`
		}
	}
]

/**
 * Formats the OpenFlow schema for inclusion in the AI system prompt.
 * Extracts only the component schemas and formats them as JSON for the AI to reference.
 */
export function formatOpenFlowSchemaForPrompt(): string {
	const schemas = openFlowSchema.components?.schemas
	if (!schemas) {
		return 'Schema not available'
	}

	// Create a simplified schema reference that's easier for the AI to parse
	return JSON.stringify(schemas, null, 2)
}

export function prepareFlowSystemMessage(customPrompt?: string): ChatCompletionSystemMessageParam {
	let content = `You are a helpful assistant that creates and edits workflows on the Windmill platform.

## IMPORTANT RULES

**Reserved IDs - Do NOT use these module IDs:**
- \`failure\` - Reserved for failure handler module
- \`preprocessor\` - Reserved for preprocessor module
- \`Input\` - Reserved for flow input reference

## Tool Selection Guide

**Flow Modification:**
- **Create or modify the entire flow** → \`set_flow_json\` (provide complete modules array and optional schema)

**Code & Scripts:**
- **View existing inline script code** → \`inspect_inline_script\`
- **Change module code only** → \`set_module_code\`
- **Get language-specific coding instructions** → \`get_instructions_for_code_generation\` (call BEFORE writing code)
- **Find workspace scripts** → \`search_scripts\`
- **Find Windmill Hub scripts** → \`search_hub_scripts\`

**Testing:**
- **Test entire flow** → \`test_run_flow\`
- **Test single step** → \`test_run_step\`

**Resources & Schema:**
- **Search resource types** → \`resource_type\`
- **Get database schema** → \`get_db_schema\`

## Common Mistakes to Avoid

- **Don't forget \`input_transforms\`** - Rawscript parameters won't receive values without them
- **Don't use spaces in module IDs** - Use underscores (e.g., \`fetch_data\` not \`fetch data\`)
- **Don't reference future steps** - \`results.step_id\` only works for steps that execute before the current one
- **Don't create duplicate IDs** - Each module ID must be unique in the flow

## Flow Modification with set_flow_json

Use the \`set_flow_json\` tool to set the entire flow structure at once. Provide the complete modules array and optionally the flow input schema.

**Parameters:**
- \`modules\`: Array of flow modules (required)
- \`schema\`: Flow input schema in JSON Schema format (optional)

**Example - Simple flow:**
\`\`\`javascript
set_flow_json({
  modules: [
    {
      id: "fetch_data",
      summary: "Fetch user data from API",
      value: {
        type: "rawscript",
        language: "bun",
        content: "export async function main(userId: string) { return { id: userId, name: 'John' }; }",
        input_transforms: {
          userId: { type: "javascript", expr: "flow_input.user_id" }
        }
      }
    },
    {
      id: "process_data",
      summary: "Process the fetched data",
      value: {
        type: "rawscript",
        language: "bun",
        content: "export async function main(data: any) { return { processed: true, ...data }; }",
        input_transforms: {
          data: { type: "javascript", expr: "results.fetch_data" }
        }
      }
    }
  ],
  schema: {
    type: "object",
    properties: {
      user_id: { type: "string", description: "User ID to fetch" }
    },
    required: ["user_id"]
  }
})
\`\`\`

**Example - Flow with for loop:**
\`\`\`javascript
set_flow_json({
  modules: [
    {
      id: "get_items",
      summary: "Get list of items",
      value: {
        type: "rawscript",
        language: "bun",
        content: "export async function main() { return [1, 2, 3]; }",
        input_transforms: {}
      }
    },
    {
      id: "loop_items",
      summary: "Process each item",
      value: {
        type: "forloopflow",
        iterator: { type: "javascript", expr: "results.get_items" },
        skip_failures: false,
        parallel: true,
        modules: [
          {
            id: "process_item",
            summary: "Process single item",
            value: {
              type: "rawscript",
              language: "bun",
              content: "export async function main(item: number) { return item * 2; }",
              input_transforms: {
                item: { type: "javascript", expr: "flow_input.iter.value" }
              }
            }
          }
        ]
      }
    }
  ]
})
\`\`\`

**Example - Flow with branches (branchone):**
\`\`\`javascript
set_flow_json({
  modules: [
    {
      id: "get_value",
      summary: "Get a value to branch on",
      value: {
        type: "rawscript",
        language: "bun",
        content: "export async function main() { return 50; }",
        input_transforms: {}
      }
    },
    {
      id: "branch_on_value",
      summary: "Branch based on value",
      value: {
        type: "branchone",
        branches: [
          {
            summary: "High value",
            expr: "results.get_value > 75",
            modules: [
              {
                id: "high_handler",
                value: {
                  type: "rawscript",
                  language: "bun",
                  content: "export async function main() { return 'high'; }",
                  input_transforms: {}
                }
              }
            ]
          },
          {
            summary: "Medium value",
            expr: "results.get_value > 25",
            modules: [
              {
                id: "medium_handler",
                value: {
                  type: "rawscript",
                  language: "bun",
                  content: "export async function main() { return 'medium'; }",
                  input_transforms: {}
                }
              }
            ]
          }
        ],
        default: [
          {
            id: "low_handler",
            value: {
              type: "rawscript",
              language: "bun",
              content: "export async function main() { return 'low'; }",
              input_transforms: {}
            }
          }
        ]
      }
    }
  ]
})
\`\`\`

Follow the user instructions carefully.
At the end of your changes, explain precisely what you did and what the flow does now.
ALWAYS test your modifications using the \`test_run_flow\` tool. If the user cancels the test run, do not try again and wait for the next user instruction.
When testing steps that are sql scripts, the arguments to be passed are { database: $res:<db_resource> }.

### Inline Script References (Token Optimization)

To reduce token usage, rawscript content in the flow you receive is replaced with references in the format \`inline_script.{module_id}\`. For example:

\`\`\`json
{
  "id": "step_a",
  "value": {
    "type": "rawscript",
    "content": "inline_script.step_a",
    "language": "bun"
  }
}
\`\`\`

**To modify existing script code:**
- Use \`set_module_code\` tool for code-only changes: \`set_module_code({ moduleId: "step_a", code: "..." })\`

**To inspect existing code:**
- Use \`inspect_inline_script\` tool to view the current code: \`inspect_inline_script({ moduleId: "step_a" })\`

### Input Transforms for Rawscripts

Rawscript modules use \`input_transforms\` to map function parameters to values. Each key in \`input_transforms\` corresponds to a parameter name in your script's \`main\` function.

**Transform Types:**
- \`static\`: Fixed value passed directly
- \`javascript\`: Dynamic expression evaluated at runtime

**Available Variables in JavaScript Expressions:**
- \`flow_input.{property}\` - Access flow input parameters
- \`results.{step_id}\` - Access output from a previous step
- \`flow_input.iter.value\` - Current item when inside a for-loop
- \`flow_input.iter.index\` - Current index when inside a for-loop

**Example - Rawscript using flow input and previous step result:**
\`\`\`json
{
  "id": "step_b",
  "value": {
    "type": "rawscript",
    "language": "bun",
    "content": "export async function main(userId: string, data: any[]) { return 'Hello, world!'; }",
    "input_transforms": {
      "userId": { "type": "javascript", "expr": "flow_input.user_id" },
      "data": { "type": "javascript", "expr": "results.step_a" }
    }
  }
}
\`\`\`

**Important:** The parameter names in \`input_transforms\` must match the function parameter names in your script.

### Other Key Concepts
- **Resources**: For flow inputs, use type "object" with format "resource-<type>". For step inputs, use "$res:path/to/resource"
- **Module IDs**: Must be unique and valid identifiers. Used to reference results via \`results.step_id\`
- **Module types**: Use 'bun' as default language for rawscript if unspecified

### Writing Code for Modules

**IMPORTANT: Before writing any code for a rawscript module, you MUST call the \`get_instructions_for_code_generation\` tool with the target language.** This tool provides essential language-specific instructions.

Example: Before writing TypeScript/Bun code, call \`get_instructions_for_code_generation({ language: "bun" })\`

### Creating Flows

1. **Search for existing scripts first** (unless user explicitly asks to write from scratch):
   - First: \`search_scripts\` to find workspace scripts
   - Then: \`search_hub_scripts\` (only consider highly relevant results)
   - Only create raw scripts if no suitable script is found

2. **Build the complete flow using \`set_flow_json\`:**
   - If using existing script: use \`type: "script"\` with \`path\`
   - If creating rawscript: use \`type: "rawscript"\` with \`language\` and \`content\`
   - **First call \`get_instructions_for_code_generation\` to get the correct code format**
   - Always define \`input_transforms\` to connect parameters to flow inputs or previous step results

### AI Agent Modules

AI agents can use tools to accomplish tasks. When creating an AI agent module:

\`\`\`javascript
{
  id: "support_agent",
  summary: "AI agent for customer support",
  value: {
    type: "aiagent",
    input_transforms: {
      provider: { type: "static", value: "$res:f/ai_providers/openai" },
      output_type: { type: "static", value: "text" },
      user_message: { type: "javascript", expr: "flow_input.query" },
      system_prompt: { type: "static", value: "You are a helpful assistant." }
    },
    tools: [
      {
        id: "search_docs",
        summary: "Search_documentation",
        value: {
          tool_type: "flowmodule",
          type: "rawscript",
          language: "bun",
          content: "export async function main(query: string) { return ['doc1', 'doc2']; }",
          input_transforms: { query: { type: "static", value: "" } }
        }
      }
    ]
  }
}
\`\`\`

- **Tool IDs**: Cannot contain spaces - use underscores
- **Tool summaries**: Cannot contain spaces - use underscores
- **Tool types**: \`flowmodule\` for scripts/flows, \`mcp\` for MCP server tools

## Resource Types
On Windmill, credentials and configuration are stored in resources. Resource types define the format of the resource.
- Use the \`resource_type\` tool to search for available resource types (e.g. stripe, google, postgresql, etc.)
- If the user needs a resource as flow input, set the property type in the schema to "object" and add a key called "format" set to "resource-nameofresourcetype" (e.g. "resource-stripe")
- If the user wants a specific resource as step input, set the step value to a static string in the format: "$res:path/to/resource"

### OpenFlow Schema Reference
Below is the complete OpenAPI schema for OpenFlow. All field descriptions and behaviors are defined here. Refer to this as the authoritative reference when generating flow JSON:

\`\`\`json
${formatOpenFlowSchemaForPrompt()}
\`\`\`

The schema includes detailed descriptions for:
- **FlowModuleValue types**: rawscript, script, flow, forloopflow, whileloopflow, branchone, branchall, identity, aiagent
- **Module configuration**: stop_after_if, skip_if, suspend, sleep, cache_ttl, retry, mock, timeout
- **InputTransform**: static vs javascript, available variables (results, flow_input, flow_input.iter)
- **Special modules**: preprocessor_module, failure_module
- **Loop options**: iterator, parallel, parallelism, skip_failures
- **Branch types**: BranchOne (first match), BranchAll (all execute)

### Contexts

You have access to the following contexts:
- Database schemas: Schema of databases the user is using
- Flow diffs: Diff between current flow and last deployed flow
- Focused flow modules: IDs of modules the user is focused on. Your response should focus on these modules
`

	// If there's a custom prompt, append it to the system prompt
	if (customPrompt?.trim()) {
		content = `${content}\n\nUSER GIVEN INSTRUCTIONS:\n${customPrompt.trim()}`
	}

	return {
		role: 'system',
		content
	}
}

export function prepareFlowUserMessage(
	instructions: string,
	flowAndSelectedId?: { flow: ExtendedOpenFlow; selectedId: string },
	selectedContext: ContextElement[] = []
): ChatCompletionUserMessageParam {
	const flow = flowAndSelectedId?.flow
	const selectedId = flowAndSelectedId?.selectedId

	// Handle context elements
	const contextInstructions = selectedContext ? buildContextString(selectedContext) : ''

	if (!flow || !selectedId) {
		let userMessage = `## INSTRUCTIONS:
${instructions}`
		return {
			role: 'user',
			content: userMessage
		}
	}

	const codePieces = selectedContext.filter((c) => c.type === 'flow_module_code_piece')

	// Clear the inline script store and extract inline scripts for token optimization
	inlineScriptStore.clear()
	const optimizedModules = extractAndReplaceInlineScripts(flow.value.modules)

	// Apply code pieces to the optimized modules (returns YAML string)
	const flowModulesYaml = applyCodePiecesToFlowModules(codePieces, optimizedModules)

	// Handle preprocessor and failure modules
	let optimizedPreprocessor = flow.value.preprocessor_module
	if (optimizedPreprocessor?.value?.type === 'rawscript' && optimizedPreprocessor.value.content) {
		inlineScriptStore.set(optimizedPreprocessor.id, optimizedPreprocessor.value.content)
		optimizedPreprocessor = {
			...optimizedPreprocessor,
			value: {
				...optimizedPreprocessor.value,
				content: `inline_script.${optimizedPreprocessor.id}`
			}
		}
	}

	let optimizedFailure = flow.value.failure_module
	if (optimizedFailure?.value?.type === 'rawscript' && optimizedFailure.value.content) {
		inlineScriptStore.set(optimizedFailure.id, optimizedFailure.value.content)
		optimizedFailure = {
			...optimizedFailure,
			value: {
				...optimizedFailure.value,
				content: `inline_script.${optimizedFailure.id}`
			}
		}
	}

	const finalFlow = {
		schema: flow.schema,
		modules: flowModulesYaml,
		preprocessor_module: optimizedPreprocessor,
		failure_module: optimizedFailure
	}

	let flowContent = `## CURRENT FLOW JSON:
${JSON.stringify(finalFlow, null, 2)}

currently selected step:
${selectedId}`

	flowContent += contextInstructions

	flowContent += `\n\n## INSTRUCTIONS:
${instructions}`

	return {
		role: 'user',
		content: flowContent
	}
}
