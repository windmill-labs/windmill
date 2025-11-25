import { ScriptService, type FlowModule, type RawScript, type Script, JobService } from '$lib/gen'
import type {
	ChatCompletionSystemMessageParam,
	ChatCompletionUserMessageParam
} from 'openai/resources/chat/completions.mjs'
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
	findModuleById
} from '../shared'
import type { ContextElement } from '../context'
import type { ExtendedOpenFlow } from '$lib/components/flows/types'
import openFlowSchema from './openFlow.json'

/**
 * Action types for flow module changes during diff tracking
 * - added: Module was added to the flow
 * - modified: Module content was changed
 * - removed: Module was deleted from the flow
 * - shadowed: Module is shown as removed (visualization mode)
 */
export type AIModuleAction = 'added' | 'modified' | 'removed' | 'shadowed' | undefined

/**
 * Tracks the action performed on a module and whether it requires user approval
 */
export type ModuleActionInfo = {
	action: AIModuleAction
	/** Whether this change is pending user approval (accept/reject) */
	pending: boolean
}

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
	/** Set a snapshot of the flow before changes for diff tracking */
	setLastSnapshot: (snapshot: ExtendedOpenFlow) => void
	/** Revert the entire flow to a previous snapshot */
	revertToSnapshot: (snapshot?: ExtendedOpenFlow) => void

	// ai chat tools
	setCode: (id: string, code: string) => Promise<void>
	setFlowJson: (json: string) => Promise<void>
	getFlowInputsSchema: () => Promise<Record<string, any>>

	// accept/reject operations (via flowDiffManager)
	/** Accept all pending module changes */
	acceptAllModuleActions: () => void
	/** Reject all pending module changes */
	rejectAllModuleActions: () => void
	/** Check if there are pending changes requiring user approval */
	hasPendingChanges: () => boolean
	/** Select a step in the flow */
	selectStep: (id: string) => void
}

const searchScriptsSchema = z.object({
	query: z
		.string()
		.describe('The query to search for, e.g. send email, list stripe invoices, etc..')
})

const searchScriptsToolDef = createToolDef(
	searchScriptsSchema,
	'search_scripts',
	'Search for scripts in the workspace'
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
	'Search for resource types'
)

const getInstructionsForCodeGenerationToolSchema = z.object({
	id: z.string().describe('The id of the step to generate code for'),
	language: langSchema.describe('The programming language the code will be written in')
})

const getInstructionsForCodeGenerationToolDef = createToolDef(
	getInstructionsForCodeGenerationToolSchema,
	'get_instructions_for_code_generation',
	'Get instructions for code generation for a raw script step'
)

const setFlowJsonSchema = z.object({
	json: z
		.string()
		.describe(
			'Complete flow JSON including modules array, and optionally schema (for flow inputs), preprocessor_module and failure_module'
		)
})

const setFlowJsonToolDef = createToolDef(
	setFlowJsonSchema,
	'set_flow_json',
	'Set the entire flow structure using JSON. Use this for changes to the flow structure and/or input schema. The JSON should include the complete modules array, and optionally schema (for flow inputs), preprocessor_module and failure_module. All existing modules will be replaced.'
)

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
		.nullable()
		.optional()
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
	'Inspect the full content of an inline script that was replaced with a reference. Use this when you need to see or modify the actual script code for a specific module.'
)

const setModuleCodeSchema = z.object({
	moduleId: z.string().describe('The ID of the module to set code for'),
	code: z.string().describe('The full script code content')
})

const setModuleCodeToolDef = createToolDef(
	setModuleCodeSchema,
	'set_module_code',
	'Set or modify the code for an existing inline script module. Use this to modify code without needing to call set_flow_json. The module must already exist in the flow.'
)

const workspaceScriptsSearch = new WorkspaceScriptsSearch()

/**
 * Storage for inline scripts extracted from flow modules.
 * Maps module IDs to their rawscript content for token-efficient transmission to AI.
 */
class InlineScriptStore {
	private scripts: Map<string, string> = new Map()

	clear() {
		this.scripts.clear()
	}

	set(moduleId: string, content: string) {
		this.scripts.set(moduleId, content)
	}

	get(moduleId: string): string | undefined {
		return this.scripts.get(moduleId)
	}

	has(moduleId: string): boolean {
		return this.scripts.has(moduleId)
	}

	getAll(): Record<string, string> {
		return Object.fromEntries(this.scripts.entries())
	}
}

const inlineScriptStore = new InlineScriptStore()

/**
 * Recursively extracts all rawscript content from flow modules and stores them.
 * Replaces the content with references like "inline_script.{module_id}".
 */
function extractAndReplaceInlineScripts(modules: FlowModule[]): FlowModule[] {
	if (!modules || !Array.isArray(modules)) {
		return []
	}

	return modules.map((module) => {
		const newModule = { ...module }

		if (newModule.value.type === 'rawscript' && newModule.value.content) {
			// Store the original content
			inlineScriptStore.set(module.id, newModule.value.content)

			// Replace with reference
			newModule.value = {
				...newModule.value,
				content: `inline_script.${module.id}`
			}
		} else if (newModule.value.type === 'forloopflow' || newModule.value.type === 'whileloopflow') {
			// Recursively process nested modules in loops
			if (newModule.value.modules) {
				newModule.value = {
					...newModule.value,
					modules: extractAndReplaceInlineScripts(newModule.value.modules)
				}
			}
		} else if (newModule.value.type === 'branchone') {
			// Process branches and default modules
			if (newModule.value.branches) {
				newModule.value = {
					...newModule.value,
					branches: newModule.value.branches.map((branch) => ({
						...branch,
						modules: branch.modules ? extractAndReplaceInlineScripts(branch.modules) : []
					}))
				}
			}
			if (newModule.value.default) {
				newModule.value = {
					...newModule.value,
					default: extractAndReplaceInlineScripts(newModule.value.default)
				}
			}
		} else if (newModule.value.type === 'branchall') {
			// Process all branches
			if (newModule.value.branches) {
				newModule.value = {
					...newModule.value,
					branches: newModule.value.branches.map((branch) => ({
						...branch,
						modules: branch.modules ? extractAndReplaceInlineScripts(branch.modules) : []
					}))
				}
			}
		}

		return newModule
	})
}

/**
 * Recursively restores inline script references back to their full content.
 * If content matches pattern "inline_script.{id}", looks up and restores the original.
 * If content doesn't match (new/modified script), keeps it as-is.
 */
export function restoreInlineScriptReferences(modules: FlowModule[]): FlowModule[] {
	return modules.map((module) => {
		const newModule = { ...module }

		if (newModule.value.type === 'rawscript' && newModule.value.content) {
			const content = newModule.value.content
			// Check if it's a reference
			const match = content.match(/^inline_script\.(.+)$/)
			if (match) {
				const moduleId = match[1]
				const storedContent = inlineScriptStore.get(moduleId)
				if (storedContent !== undefined) {
					// Restore original content
					newModule.value = {
						...newModule.value,
						content: storedContent
					}
				}
				// If not found in store, keep the reference as-is (shouldn't happen normally)
			}
			// If not a reference, it's new/modified content - keep as-is
		} else if (newModule.value.type === 'forloopflow' || newModule.value.type === 'whileloopflow') {
			// Recursively process nested modules in loops
			if (newModule.value.modules) {
				newModule.value = {
					...newModule.value,
					modules: restoreInlineScriptReferences(newModule.value.modules)
				}
			}
		} else if (newModule.value.type === 'branchone') {
			// Process branches and default modules
			if (newModule.value.branches) {
				newModule.value = {
					...newModule.value,
					branches: newModule.value.branches.map((branch) => ({
						...branch,
						modules: branch.modules ? restoreInlineScriptReferences(branch.modules) : []
					}))
				}
			}
			if (newModule.value.default) {
				newModule.value = {
					...newModule.value,
					default: restoreInlineScriptReferences(newModule.value.default)
				}
			}
		} else if (newModule.value.type === 'branchall') {
			// Process all branches
			if (newModule.value.branches) {
				newModule.value = {
					...newModule.value,
					branches: newModule.value.branches.map((branch) => ({
						...branch,
						modules: branch.modules ? restoreInlineScriptReferences(branch.modules) : []
					}))
				}
			}
		}

		return newModule
	})
}

/**
 * Recursively finds any unresolved inline script references in flow modules.
 * Returns array of module IDs that still have `inline_script.{id}` patterns.
 */
export function findUnresolvedInlineScriptRefs(modules: FlowModule[]): string[] {
	const unresolvedRefs: string[] = []

	function checkModule(module: FlowModule) {
		if (module.value.type === 'rawscript' && module.value.content) {
			const match = module.value.content.match(/^inline_script\.(.+)$/)
			if (match) {
				unresolvedRefs.push(match[1])
			}
		} else if (module.value.type === 'forloopflow' || module.value.type === 'whileloopflow') {
			if (module.value.modules) {
				module.value.modules.forEach(checkModule)
			}
		} else if (module.value.type === 'branchone') {
			if (module.value.branches) {
				module.value.branches.forEach((branch) => {
					branch.modules?.forEach(checkModule)
				})
			}
			if (module.value.default) {
				module.value.default.forEach(checkModule)
			}
		} else if (module.value.type === 'branchall') {
			if (module.value.branches) {
				module.value.branches.forEach((branch) => {
					branch.modules?.forEach(checkModule)
				})
			}
		}
	}

	modules.forEach(checkModule)
	return unresolvedRefs
}

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
				allowResourcesFetch: true,
				isPreprocessor: parsedArgs.id === 'preprocessor'
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
			return executeTestRun({
				jobStarter: () =>
					JobService.runFlowPreview({
						workspace: workspace,
						requestBody: {
							args: parsedArgs,
							value: flow.value
						}
					}),
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
									module.id === 'preprocessor'
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
									module.id === 'preprocessor'
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
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const parsedArgs = setModuleCodeSchema.parse(args)
			const { moduleId, code } = parsedArgs

			toolCallbacks.setToolStatus(toolId, { content: `Setting code for module '${moduleId}'...` })

			// Update store to keep it coherent (for subsequent set_flow_json calls with references)
			inlineScriptStore.set(moduleId, code)

			// Update the flow directly via helper
			await helpers.setCode(moduleId, code)

			toolCallbacks.setToolStatus(toolId, { content: `Code updated for module '${moduleId}'` })
			return `Code for module '${moduleId}' has been updated successfully.`
		}
	},
	{
		def: setFlowJsonToolDef,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const parsedArgs = setFlowJsonSchema.parse(args)
			toolCallbacks.setToolStatus(toolId, { content: 'Parsing and applying flow JSON...' })

			await helpers.setFlowJson(parsedArgs.json)

			// Check for unresolved inline script references
			const { flow } = helpers.getFlowAndSelectedId()
			const unresolvedRefs = findUnresolvedInlineScriptRefs(flow.value.modules)

			toolCallbacks.setToolStatus(toolId, { content: 'Flow JSON applied successfully' })

			if (unresolvedRefs.length > 0) {
				return `Flow structure updated with warnings: Unresolved inline script references found for modules: ${unresolvedRefs.join(', ')}. These modules have invalid content - use set_module_code to set their code.`
			}

			return 'Flow structure updated via JSON. All affected modules have been marked and require review/acceptance.'
		}
	}
]

/**
 * Formats the OpenFlow schema for inclusion in the AI system prompt.
 * Extracts only the component schemas and formats them as JSON for the AI to reference.
 */
function formatOpenFlowSchemaForPrompt(): string {
	const schemas = openFlowSchema.components?.schemas
	if (!schemas) {
		return 'Schema not available'
	}

	// Create a simplified schema reference that's easier for the AI to parse
	return JSON.stringify(schemas, null, 2)
}

export function prepareFlowSystemMessage(customPrompt?: string): ChatCompletionSystemMessageParam {
	let content = `You are a helpful assistant that creates and edits workflows on the Windmill platform. You have two main tools for modifying flows:
- **set_module_code**: Modify the code of an existing inline script module (use this for code-only changes)
- **set_flow_json**: Replace the entire flow structure with JSON (use this for structural changes like adding/removing modules)

Follow the user instructions carefully.
Go step by step, and explain what you're doing as you're doing it.
ALWAYS test your modifications. You have access to the \`test_run_flow\` and \`test_run_step\` tools to test the flow and steps. If you only modified a single step, use the \`test_run_step\` tool to test it. If you modified the flow, use the \`test_run_flow\` tool to test it. If the user cancels the test run, do not try again and wait for the next user instruction.
When testing steps that are sql scripts, the arguments to be passed are { database: $res:<db_resource> }.

## Working with JSON

The JSON must include the complete flow definition with all modules. Example structure:
\`\`\`json
{
  "schema": {
    "type": "object",
    "properties": {
      "user_id": {
        "type": "string"
      },
      "count": {
        "type": "number",
        "default": 10
      }
    },
    "required": ["user_id"]
  },
  "modules": [
    {
      "id": "step_a",
      "summary": "First step",
      "value": {
        "type": "rawscript",
        "language": "bun",
        "content": "export async function main() {...}",
        "input_transforms": {}
      }
    },
    {
      "id": "step_b",
      "value": {
        "type": "forloopflow",
        "iterator": {
          "type": "javascript",
          "expr": "results.step_a"
        },
        "skip_failures": true,
        "parallel": true,
        "modules": []
      }
    },
    {
      "id": "step_c",
      "value": {
        "type": "branchone",
        "branches": [
          {
            "expr": "results.step_a > 10",
            "modules": []
          }
        ],
        "default": []
      }
    }
  ],
  "preprocessor_module": {
    "id": "preprocessor",
    "value": {}
  },
  "failure_module": {
    "id": "failure",
    "value": {}
  }
}
\`\`\`

### Inline Script References (Token Optimization)

To reduce token usage, rawscript content in the flow JSON you receive is replaced with references in the format \`inline_script.{module_id}\`. For example:

\`\`\`json
{
  "modules": [
    {
      "id": "step_a",
      "value": {
        "type": "rawscript",
        "content": "inline_script.step_a",
        "language": "bun"
      }
    }
  ]
}
\`\`\`

**To modify an existing script's code:**
- Use the \`set_module_code\` tool: \`set_module_code(moduleId, newCode)\`
- No need to call \`set_flow_json\` for code-only changes
- If you also need structural changes, call \`set_module_code\` first, then \`set_flow_json\` with the reference

**To add a new inline script module:**
- Call \`set_flow_json\` with the full code content directly (not a reference)

**To keep existing code unchanged in structural changes:**
- Keep the \`inline_script.{module_id}\` reference as-is in \`set_flow_json\`
- The original code will be restored automatically

**To inspect existing code:**
- Use \`inspect_inline_script\` tool to view current code before modifying

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
    "content": "export async function main(userId: string, data: any[]) { ... }",
    "input_transforms": {
      "userId": {
        "type": "javascript",
        "expr": "flow_input.user_id"
      },
      "data": {
        "type": "javascript",
        "expr": "results.step_a"
      }
    }
  }
}
\`\`\`

**Example - Static value:**
\`\`\`json
{
  "input_transforms": {
    "limit": {
      "type": "static",
      "value": 100
    }
  }
}
\`\`\`

**Important:** The parameter names in \`input_transforms\` must match the function parameter names in your script. When you create or modify a rawscript, always define \`input_transforms\` to connect it to flow inputs or results from other steps.

### Other Key Concepts
- **Resources**: For flow inputs, use type "object" with format "resource-<type>". For step inputs, use "$res:path/to/resource"
- **Module IDs**: Must be unique and valid identifiers. Used to reference results via \`results.step_id\`
- **Module types**: Use 'bun' as default language for rawscript if unspecified

### Creating New Steps
1. If the user hasn't explicitly asked to write from scratch:
   - First search for matching scripts in the workspace using \`search_scripts\`
   - Then search for matching scripts in the hub using \`search_hub_scripts\`, but ONLY consider highly relevant results
   - Only if no suitable script is found, create a raw script step
2. If found, use type \`script\` with the path
3. If creating a \`rawscript\` module:
   - If no language is specified, use 'bun' as the default language
   - Use \`get_instructions_for_code_generation\` to get the correct code format for the language
   - Create the module with inline code
4. Set appropriate \`input_transforms\` to pass data between steps
5. If any inputs use flow_input properties that don't exist yet, add them to the schema

## Resource Types
On Windmill, credentials and configuration are stored in resources. Resource types define the format of the resource.
- Use the \`resource_type\` tool to search for available resource types (e.g. stripe, google, postgresql, etc.)
- If the user needs a resource as flow input, set the property type in the schema to "object" and add a key called "format" set to "resource-nameofresourcetype" (e.g. "resource-stripe")
- If the user wants a specific resource as step input, set the step value to a static string in the format: "$res:path/to/resource"

### OpenFlow Schema Reference
Below is the complete OpenAPI schema for OpenFlow. All field descriptions and behaviors are defined here. Refer to this as the authoritative reference when generating flow YAML:

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
