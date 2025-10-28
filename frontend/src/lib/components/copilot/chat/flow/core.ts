import { ScriptService, type FlowModule, type Script, JobService } from '$lib/gen'
import type {
	ChatCompletionSystemMessageParam,
	ChatCompletionUserMessageParam
} from 'openai/resources/chat/completions.mjs'
import YAML from 'yaml'
import { z } from 'zod'
import uFuzzy from '@leeoniya/ufuzzy'
import { emptySchema, emptyString } from '$lib/utils'
import { createDbSchemaTool } from '../script/core'
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

export type AIModuleAction = 'added' | 'modified' | 'removed' | 'shadowed' | undefined

export interface FlowAIChatHelpers {
	// flow context
	getFlowAndSelectedId: () => { flow: ExtendedOpenFlow; selectedId: string }
	getPreviewFlow: () => ExtendedOpenFlow
	getModules: (id?: string) => FlowModule[]
	getFlowInputsSchema: () => Promise<Record<string, any>>
	// flow diff management
	hasDiff: () => boolean
	setLastSnapshot: (snapshot: ExtendedOpenFlow) => void
	showModuleDiff: (id: string) => void
	getModuleAction: (id: string) => AIModuleAction | undefined
	revertModuleAction: (id: string) => void
	acceptModuleAction: (id: string) => void
	acceptAllModuleActions: () => void
	rejectAllModuleActions: () => void
	revertToSnapshot: (snapshot?: ExtendedOpenFlow) => void
	// ai chat tools
	selectStep: (id: string) => void
	setFlowYaml: (yaml: string) => Promise<void>
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

const setFlowYamlSchema = z.object({
	yaml: z
		.string()
		.describe(
			'Complete flow YAML including modules array, and optionally preprocessor_module and failure_module'
		)
})

const setFlowYamlToolDef = createToolDef(
	setFlowYamlSchema,
	'set_flow_yaml',
	'Set the entire flow structure using YAML. Use this for complex multi-step changes where multiple modules need to be added, removed, or reorganized. The YAML should include the complete modules array, and optionally preprocessor_module and failure_module. All existing modules will be replaced.'
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
		def: setFlowYamlToolDef,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const parsedArgs = setFlowYamlSchema.parse(args)
			toolCallbacks.setToolStatus(toolId, { content: 'Parsing and applying flow YAML...' })

			await helpers.setFlowYaml(parsedArgs.yaml)

			toolCallbacks.setToolStatus(toolId, { content: 'Flow YAML applied successfully' })

			return 'Flow structure updated via YAML. All affected modules have been marked and require review/acceptance.'
		},
		requiresConfirmation: true,
		confirmationMessage: 'Apply flow YAML changes',
		showDetails: true
	}
]

export function prepareFlowSystemMessage(customPrompt?: string): ChatCompletionSystemMessageParam {
	let content = `You are a helpful assistant that creates and edits workflows on the Windmill platform. You're provided with a bunch of tools to help you edit the flow.
Follow the user instructions carefully.
Go step by step, and explain what you're doing as you're doing it.
DO NOT wait for user confirmation before performing an action. Only do it if the user explicitly asks you to wait in their initial instructions.
ALWAYS test your modifications. You have access to the \`test_run_flow\` and \`test_run_step\` tools to test the flow and steps. If you only modified a single step, use the \`test_run_step\` tool to test it. If you modified the flow, use the \`test_run_flow\` tool to test it. If the user cancels the test run, do not try again and wait for the next user instruction.
When testing steps that are sql scripts, the arguments to be passed are { database: $res:<db_resource> }.

## Modifying Flows with YAML

You modify flows by using the **set_flow_yaml** tool. This tool replaces the entire flow structure with the YAML you provide.

### When to Make Changes
When the user requests modifications to the flow structure (adding steps, removing steps, reorganizing, changing configurations, etc.), use the set_flow_yaml tool to apply all changes at once.

### YAML Structure
The YAML must include the complete flow definition:
\`\`\`yaml
modules:
  - id: step_a
    summary: "First step"
    value:
      type: rawscript
      language: bun
      content: "export async function main() {...}"
      input_transforms: {}
  - id: step_b
    summary: "Second step"
    value:
      type: forloopflow
      iterator:
        type: javascript
        expr: "results.step_a"
      skip_failures: true
      parallel: true
      parallelism: 10
      modules:
        - id: step_b_a
          value:
            type: rawscript
            ...
  - id: step_c
    summary: "Branch logic"
    value:
      type: branchone
      branches:
        - summary: "First condition"
          expr: "results.step_a > 10"
          modules: [...]
        - summary: "Second condition"
          expr: "results.step_a <= 10"
          modules: [...]
      default: {...}  # optional default branch
preprocessor_module:  # optional
  id: preprocessor
  value:
    type: rawscript
    ...
failure_module:  # optional
  id: failure
  value:
    type: rawscript
    ...
\`\`\`

### Module Types
- **rawscript**: Inline code (use 'bun' as default language if unspecified)
- **script**: Reference to existing script by path
- **flow**: Reference to existing flow by path
- **forloopflow**: For loop with nested modules
- **branchone**: Conditional branches (only first matching executes)
- **branchall**: Parallel branches (all execute)

### Module Configuration Options
All modules can have these fields in their \`value\`:
- **input_transforms**: Object mapping input names to JavaScript expressions
  - Use \`results.step_id\` to reference previous step results
  - Use \`flow_input.property\` to reference flow inputs
  - Use \`flow_input.iter.value\` inside loops for the current iteration value
- **stop_after_if**: Object with \`expr\` and \`skip_if_stopped\` for early termination
  - Expression can use \`flow_input\` or \`result\` (the step's own result)
  - Example: \`{ expr: "result.status === 'done'", skip_if_stopped: false }\`
- **skip_if**: Object with \`expr\` to conditionally skip the module
  - Expression can use \`flow_input\` or \`results.<step_id>\`
  - Example: \`{ expr: "results.step_a === null" }\`
- **suspend**: Suspend configuration for approval steps
- **sleep**: Sleep configuration
- **cache_ttl**: Cache duration in seconds
- **retry**: Retry configuration
- **mock**: Mock configuration for testing

### For Loop Options
For \`forloopflow\` modules, configure these options:
- **iterator**: Object with \`type: "javascript"\` and \`expr\` (the expression to iterate over)
- **parallel**: Boolean, run iterations in parallel (faster for I/O operations)
- **parallelism**: Number, limit concurrent iterations when parallel=true
- **skip_failures**: Boolean, continue on iteration failures

### Special Modules
- **Preprocessor** (\`preprocessor_module\`): Runs before first step on external triggers
  - Must have \`id: "preprocessor"\`
  - Only supports script/rawscript types
  - Cannot reference other step results
- **Failure Handler** (\`failure_module\`): Runs when flow fails
  - Must have \`id: "failure"\`
  - Only supports script/rawscript types
  - Can access error object: \`{ message, name, stack, step_id }\`

### Creating New Steps
When creating new steps:
1. Search for existing scripts using \`search_scripts\` or \`search_hub_scripts\` tools
2. If found, use type \`script\` with the path
3. If not found, create a \`rawscript\` module with inline code
4. Set appropriate \`input_transforms\` to pass data between steps

### Flow Input Schema
The flow's input schema is defined separately in the flow object (not in YAML). When using \`flow_input\` properties, ensure they exist in the schema. For resource inputs, use:
- Type: "object"
- Format: "resource-<type>" (e.g., "resource-stripe")

### Static Resource References
To reference a specific resource in input_transforms, use: \`"$res:path/to/resource"\`

### Important Notes
- The YAML must include the **complete modules array**, not just changed modules
- Module IDs must be unique and valid identifiers (alphanumeric, underscore, hyphen)
- Steps execute in the order they appear in the modules array
- After applying, all modules are marked for review and displayed in a diff view
- This tool requires user confirmation before execution

### Contexts

You have access to the following contexts:
- Database schemas
- Flow diffs
- Focused flow modules
Database schemas give you the schema of databases the user is using.
Flow diffs give you the diff between the current flow and the last deployed flow.
Focused flow modules give you the ids of the flow modules the user is focused on. Your response should focus on these modules.

## Resource types
On Windmill, credentials and configuration are stored in resources. Resource types define the format of the resource.
If the user needs a resource as flow input, you should set the property type in the schema to "object" as well as add a key called "format" and set it to "resource-nameofresourcetype" (e.g. "resource-stripe").
If the user wants a specific resource as step input, you should set the step value to a static string in the following format: "$res:path/to/resource".
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
	const flowModulesYaml = applyCodePiecesToFlowModules(codePieces, flow.value.modules)

	let flowContent = `## FLOW:
flow_input schema:
${JSON.stringify(flow.schema ?? emptySchema())}

flow modules:
${flowModulesYaml}

preprocessor module:
${YAML.stringify(flow.value.preprocessor_module)}

failure module:
${YAML.stringify(flow.value.failure_module)}

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
