import { ScriptService, type FlowModule, type RawScript, type Script } from '$lib/gen'
import type {
	ChatCompletionSystemMessageParam,
	ChatCompletionUserMessageParam
} from 'openai/resources/chat/completions.mjs'
import YAML from 'yaml'
import { z } from 'zod'
import uFuzzy from '@leeoniya/ufuzzy'
import { emptySchema, emptyString } from '$lib/utils'
import {
	getFormattedResourceTypes,
	getLangContext,
	SUPPORTED_CHAT_SCRIPT_LANGUAGES
} from '../script/core'
import { createSearchHubScriptsTool, createToolDef, type Tool } from '../shared'
import type { ExtendedOpenFlow } from '$lib/components/flows/types'

export type AIModuleAction = 'added' | 'modified' | 'removed'

export interface FlowAIChatHelpers {
	// flow context
	getFlowAndSelectedId: () => { flow: ExtendedOpenFlow; selectedId: string }
	// flow apply/reject
	getPreviewFlow: () => ExtendedOpenFlow
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
	insertStep: (location: InsertLocation, step: NewStep) => Promise<string>
	removeStep: (id: string) => void
	getStepInputs: (id: string) => Promise<Record<string, any>>
	setStepInputs: (id: string, inputs: string) => Promise<void>
	getFlowInputsSchema: () => Promise<Record<string, any>>
	setFlowInputsSchema: (inputs: Record<string, any>) => Promise<void>
	selectStep: (id: string) => void
	getStepCode: (id: string) => string
	getModules: (id?: string) => FlowModule[]
	setBranchPredicate: (id: string, branchIndex: number, expression: string) => Promise<void>
	addBranch: (id: string) => Promise<void>
	removeBranch: (id: string, branchIndex: number) => Promise<void>
	setForLoopIteratorExpression: (id: string, expression: string) => Promise<void>
	setCode: (id: string, code: string) => Promise<void>
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

const newStepSchema = z.union([
	z
		.object({
			type: z.literal('rawscript'),
			language: langSchema.describe(
				'The language to use for the code, default to bun if none specified'
			),
			summary: z.string().describe('The summary of what the step does, in 3-5 words')
		})
		.describe('Add a raw script step at the specified location'),
	z
		.object({
			type: z.literal('script'),
			path: z.string().describe('The path of the script to use for the step.')
		})
		.describe('Add a script step at the specified location'),
	z
		.object({
			type: z.literal('forloop')
		})
		.describe('Add a for loop at the specified location'),
	z
		.object({
			type: z.literal('branchall')
		})
		.describe('Add a branch all at the specified location: all branches will be executed'),
	z
		.object({
			type: z.literal('branchone')
		})
		.describe(
			'Add a branch one at the specified location: only the first branch that evaluates to true will be executed'
		)
])

type NewStep = z.infer<typeof newStepSchema>

const insertLocationSchema = z.union([
	z
		.object({
			type: z.literal('after'),
			afterId: z.string().describe('The id of the step after which the new step will be added.')
		})
		.describe('Add a step after the given step id'),
	z
		.object({
			type: z.literal('start')
		})
		.describe('Add a step at the start of the flow'),
	z
		.object({
			type: z.literal('start_inside_forloop'),
			inside: z
				.string()
				.describe('The id of the step inside which the new step will be added (forloop step only)')
		})
		.describe('Add a step at the start of the given step (forloop step only)'),
	z
		.object({
			type: z.literal('start_inside_branch'),
			inside: z
				.string()
				.describe(
					'The id of the step inside which the new step will be added (branchone or branchall only).'
				),
			branchIndex: z
				.number()
				.describe(
					'The index of the branch inside the forloop step, starting at 0. For the default branch (branchone only), the branch index is -1.'
				)
		})
		.describe(
			'Add a step at the start of a given branch of the given step (branchone or branchall only)'
		),
	z
		.object({
			type: z.literal('preprocessor')
		})
		.describe('Insert a preprocessor step (runs before the first step when triggered externally)'),
	z
		.object({
			type: z.literal('failure')
		})
		.describe('Insert a failure step (only executed when the flow fails)')
])

type InsertLocation = z.infer<typeof insertLocationSchema>

const addStepSchema = z.object({
	location: insertLocationSchema,
	step: newStepSchema
})

const addStepToolDef = createToolDef(
	addStepSchema,
	'add_step',
	'Add a step at the specified location'
)

const removeStepSchema = z.object({
	id: z.string().describe('The id of the step to remove')
})

const removeStepToolDef = createToolDef(
	removeStepSchema,
	'remove_step',
	'Remove the step with the given id'
)

const setForLoopIteratorExpressionSchema = z.object({
	id: z.string().describe('The id of the forloop step to set the iterator expression for'),
	expression: z.string().describe('The JavaScript expression to set for the iterator')
})

const setForLoopIteratorExpressionToolDef = createToolDef(
	setForLoopIteratorExpressionSchema,
	'set_forloop_iterator_expression',
	'Set the iterator JavaScript expression for the given forloop step'
)

const setBranchPredicateSchema = z.object({
	id: z.string().describe('The id of the branchone step to set the predicates for'),
	branchIndex: z
		.number()
		.describe('The index of the branch to set the predicate for, starting at 0.'),
	expression: z.string().describe('The JavaScript expression to set for the predicate')
})
const setBranchPredicateToolDef = createToolDef(
	setBranchPredicateSchema,
	'set_branch_predicate',
	'Set the predicates using a JavaScript expression for the given branch, only applicable for branchone branches.'
)

const addBranchSchema = z.object({
	id: z.string().describe('The id of the step to add the branch to')
})
const addBranchToolDef = createToolDef(
	addBranchSchema,
	'add_branch',
	'Add a branch to the given step, applicable to branchall and branchone steps'
)

const removeBranchSchema = z.object({
	id: z.string().describe('The id of the step to remove the branch from'),
	branchIndex: z.number().describe('The index of the branch to remove, starting at 0')
})
const removeBranchToolDef = createToolDef(
	removeBranchSchema,
	'remove_branch',
	'Remove the branch with the given index from the given step, applicable to branchall and branchone steps.'
)

const getStepInputsSchema = z.object({
	id: z.string().describe('The id of the step to get the inputs for')
})

const getStepInputsToolDef = createToolDef(
	getStepInputsSchema,
	'get_step_inputs',
	'Get the inputs for the given step id'
)

const setStepInputsSchema = z.object({
	id: z.string().describe('The id of the step to set the inputs for'),
	inputs: z.string().describe('The inputs to set for the step')
})

const setStepInputsToolDef = createToolDef(
	setStepInputsSchema,
	'set_step_inputs',
	`Set all inputs for the given step id. 

Return a list of input. Each input should be defined by its input name enclosed in double square brackets ([[inputName]]), followed by a JavaScript expression that sets its value.
The value expression can span multiple lines. Separate each input block with a blank line.

Example:

[[input1]]
\`Hello, \${results.a}\`

[[input2]]
flow_input.iter.value

[[input3]]
flow_input.x`
)

const setFlowInputsSchemaSchema = z.object({
	schema: z.string().describe('JSON string of the flow inputs schema (draft 2020-12)')
})

const setFlowInputsSchemaToolDef = createToolDef(
	setFlowInputsSchemaSchema,
	'set_flow_inputs_schema',
	'Set the flow inputs schema. **Overrides the current schema.**'
)

const setCodeSchema = z.object({
	id: z.string().describe('The id of the step to set the code for'),
	code: z.string().describe('The code to apply')
})

const setCodeToolDef = createToolDef(
	setCodeSchema,
	'set_code',
	'Set the code for the current step.'
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

const workspaceScriptsSearch = new WorkspaceScriptsSearch()

export const flowTools: Tool<FlowAIChatHelpers>[] = [
	createSearchHubScriptsTool(false),
	{
		def: searchScriptsToolDef,
		fn: async ({ args, workspace, toolId, toolCallbacks }) => {
			toolCallbacks.setToolStatus(
				toolId,
				'Searching for workspace scripts related to "' + args.query + '"...'
			)
			const parsedArgs = searchScriptsSchema.parse(args)
			const scriptResults = await workspaceScriptsSearch.search(parsedArgs.query, workspace)
			toolCallbacks.setToolStatus(
				toolId,
				'Found ' +
					scriptResults.length +
					' scripts in the workspace related to "' +
					args.query +
					'"'
			)
			return JSON.stringify(scriptResults)
		}
	},
	{
		def: addStepToolDef,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const parsedArgs = addStepSchema.parse(args)
			toolCallbacks.setToolStatus(
				toolId,
				parsedArgs.location.type === 'after'
					? `Adding a step after step '${parsedArgs.location.afterId}'`
					: parsedArgs.location.type === 'start'
						? 'Adding a step at the start'
						: parsedArgs.location.type === 'start_inside_forloop'
							? `Adding a step at the start of the forloop step '${parsedArgs.location.inside}'`
							: parsedArgs.location.type === 'start_inside_branch'
								? `Adding a step at the start of the branch ${parsedArgs.location.branchIndex + 1} of step '${parsedArgs.location.inside}'`
								: parsedArgs.location.type === 'preprocessor'
									? 'Adding a preprocessor step'
									: parsedArgs.location.type === 'failure'
										? 'Adding a failure step'
										: 'Adding a step'
			)
			const id = await helpers.insertStep(parsedArgs.location, parsedArgs.step)
			helpers.selectStep(id)

			toolCallbacks.setToolStatus(toolId, `Added step '${id}'`)

			return `Step ${id} added. Here is the updated flow, make sure to take it into account when adding another step:\n${YAML.stringify(helpers.getModules())}`
		}
	},
	{
		def: removeStepToolDef,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			toolCallbacks.setToolStatus(toolId, `Removing step ${args.id}...`)
			const parsedArgs = removeStepSchema.parse(args)
			helpers.removeStep(parsedArgs.id)
			toolCallbacks.setToolStatus(toolId, `Removed step '${parsedArgs.id}'`)
			return `Step '${parsedArgs.id}' removed. Here is the updated flow:\n${YAML.stringify(helpers.getModules())}`
		}
	},
	{
		def: getStepInputsToolDef,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			toolCallbacks.setToolStatus(toolId, `Getting step ${args.id} inputs...`)
			const parsedArgs = getStepInputsSchema.parse(args)
			const inputs = await helpers.getStepInputs(parsedArgs.id)
			toolCallbacks.setToolStatus(toolId, `Retrieved step '${parsedArgs.id}' inputs`)
			return YAML.stringify(inputs)
		}
	},
	{
		def: setStepInputsToolDef,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			toolCallbacks.setToolStatus(toolId, `Setting step ${args.id} inputs...`)
			const parsedArgs = setStepInputsSchema.parse(args)
			await helpers.setStepInputs(parsedArgs.id, parsedArgs.inputs)
			helpers.selectStep(parsedArgs.id)
			const inputs = await helpers.getStepInputs(parsedArgs.id)
			toolCallbacks.setToolStatus(toolId, `Set step '${parsedArgs.id}' inputs`)
			return `Step '${parsedArgs.id}' inputs set. New inputs:\n${YAML.stringify(inputs)}`
		},
		preAction: ({ toolCallbacks, toolId }) => {
			toolCallbacks.setToolStatus(toolId, 'Setting step inputs...')
		}
	},
	{
		def: setFlowInputsSchemaToolDef,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			toolCallbacks.setToolStatus(toolId, 'Setting flow inputs schema...')
			const parsedArgs = setFlowInputsSchemaSchema.parse(args)
			const schema = JSON.parse(parsedArgs.schema)
			await helpers.setFlowInputsSchema(schema)
			helpers.selectStep('Input')
			const updatedSchema = await helpers.getFlowInputsSchema()
			toolCallbacks.setToolStatus(toolId, 'Set flow inputs schema')
			return `Flow inputs schema set. New schema:\n${JSON.stringify(updatedSchema)}`
		},
		preAction: ({ toolCallbacks, toolId }) => {
			toolCallbacks.setToolStatus(toolId, 'Setting flow inputs schema...')
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
			toolCallbacks.setToolStatus(
				toolId,
				'Retrieved instructions for code generation in ' + parsedArgs.language
			)
			return langContext
		}
	},
	{
		def: setCodeToolDef,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const parsedArgs = setCodeSchema.parse(args)
			toolCallbacks.setToolStatus(toolId, `Setting code for step '${parsedArgs.id}'...`)
			await helpers.setCode(parsedArgs.id, parsedArgs.code)
			helpers.selectStep(parsedArgs.id)
			toolCallbacks.setToolStatus(toolId, `Set code for step '${parsedArgs.id}'`)
			return `Step code set`
		},
		preAction: ({ toolCallbacks, toolId }) => {
			toolCallbacks.setToolStatus(toolId, 'Setting code for step...')
		}
	},
	{
		def: setBranchPredicateToolDef,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const parsedArgs = setBranchPredicateSchema.parse(args)
			await helpers.setBranchPredicate(parsedArgs.id, parsedArgs.branchIndex, parsedArgs.expression)
			helpers.selectStep(parsedArgs.id)
			toolCallbacks.setToolStatus(
				toolId,
				`Set predicate of branch ${parsedArgs.branchIndex + 1} of '${parsedArgs.id}'`
			)
			return `Branch ${parsedArgs.branchIndex} of '${parsedArgs.id}' predicate set`
		}
	},
	{
		def: addBranchToolDef,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const parsedArgs = addBranchSchema.parse(args)
			await helpers.addBranch(parsedArgs.id)
			helpers.selectStep(parsedArgs.id)
			toolCallbacks.setToolStatus(toolId, `Added branch to '${parsedArgs.id}'`)
			return `Branch added to '${parsedArgs.id}'`
		}
	},
	{
		def: removeBranchToolDef,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const parsedArgs = removeBranchSchema.parse(args)
			await helpers.removeBranch(parsedArgs.id, parsedArgs.branchIndex)
			helpers.selectStep(parsedArgs.id)
			toolCallbacks.setToolStatus(
				toolId,
				`Removed branch ${parsedArgs.branchIndex + 1} of '${parsedArgs.id}'`
			)
			return `Branch ${parsedArgs.branchIndex} of '${parsedArgs.id}' removed`
		}
	},
	{
		def: setForLoopIteratorExpressionToolDef,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const parsedArgs = setForLoopIteratorExpressionSchema.parse(args)
			await helpers.setForLoopIteratorExpression(parsedArgs.id, parsedArgs.expression)
			helpers.selectStep(parsedArgs.id)
			toolCallbacks.setToolStatus(toolId, `Set forloop '${parsedArgs.id}' iterator expression`)
			return `Forloop '${parsedArgs.id}' iterator expression set`
		}
	},
	{
		def: resourceTypeToolDef,
		fn: async ({ args, toolId, workspace, toolCallbacks }) => {
			const parsedArgs = resourceTypeToolSchema.parse(args)
			toolCallbacks.setToolStatus(
				toolId,
				'Searching resource types for "' + parsedArgs.query + '"...'
			)
			const formattedResourceTypes = await getFormattedResourceTypes(
				parsedArgs.language,
				parsedArgs.query,
				workspace
			)
			toolCallbacks.setToolStatus(toolId, 'Retrieved resource types for "' + parsedArgs.query + '"')
			return formattedResourceTypes
		}
	}
]

export function prepareFlowSystemMessage(): ChatCompletionSystemMessageParam {
	const content = `You are a helpful assistant that creates and edits workflows on the Windmill platform. You're provided with a bunch of tools to help you edit the flow.
Follow the user instructions carefully.
Go step by step, and explain what you're doing as you're doing it.
DO NOT wait for user confirmation before performing an action. Only do it if the user explicitly asks you to wait in their initial instructions.

## Understanding User Requests

### Individual Actions
When the user asks for a specific action, perform ONLY that action:
- Updating code for a step
- Setting step inputs
- Setting flow inputs schema
- Setting branch predicates
- Setting forloop iterator expressions
- Adding/removing branches
- etc.

### Full Step Creation Process
When the user asks to add one or more steps with broad instructions (e.g., "add a step to send an email", "create a flow to process data"), follow the complete process below for EACH step.

### Complete Step Creation Process
When creating new steps, follow this process for EACH step:
1. If the user hasn't explicitly asked to write from scratch:
   - First search for matching scripts in the workspace
   - Then search for matching scripts in the hub, but ONLY consider highly relevant results that match the user's requirements
   - Only if no suitable script is found, create a raw script step
2. For raw script steps:
   - If no language is specified, use 'bun' as the default language
   - Use get_instructions_for_code_generation to get the correct code format
   - Display the code to the user before setting it
   - Set the code using set_code
3. After adding any step:
   - Get the step inputs using get_step_inputs
   - Set the step inputs using set_step_inputs
   - If any inputs use flow_input properties that don't exist yet, add them to the schema using set_flow_inputs_schema

## Additional instructions for the Flow Editor

### Special Step Types
For special step types, follow these additional steps:
- For forloop steps: Set the iterator expression using set_forloop_iterator_expression
- For branchone steps: Set the predicates for each branch using set_branch_predicate
- For branchall steps: No additional setup needed

### Step Insertion Rules
When adding steps, carefully consider the execution order:
1. Steps are executed in the order they appear in the flow definition, not in the order they were added
2. For inserting steps:
   - Use 'start' to add at the beginning of the flow
   - Use 'after' with the previous step's ID to add in sequence (can be inside a branch or a forloop)
   - Use 'start_inside_forloop' to add at the start of a forloop
   - Use 'start_inside_branch' to add at the start of a branch
   - Use 'preprocessor' to add a preprocessor step
   - Use 'failure' to add a failure step
3. Always verify the flow structure after adding steps to ensure correct execution order

### Flow Inputs and Schema
- Use set_flow_inputs_schema to define or update the flow's input schema
- When using flow_input in step inputs, ensure the properties exist in the schema
- For resource inputs, set the property type to "object" and add a "format" key with value "resource-nameofresourcetype"

### JavaScript Expressions
For step inputs, forloop iterator expressions and branch predicates, use JavaScript expressions with these variables:
- Step results: results.stepid or results.stepid.property_name
- Loop iterator: flow_input.iter.value (inside loops)
- Flow inputs: flow_input.property_name
- Static values: Use JavaScript syntax (e.g., "hello", true, 3)

Note: These variables are only accessible in step inputs, forloop iterator expressions and branch predicates. They must be passed as script arguments using the set_step_inputs tool.

For truly static values in step inputs (those not linked to previous steps or loop iterations), prefer using flow inputs by default unless explicitly specified otherwise. This makes the flow more configurable and reusable. For example, instead of hardcoding an email address in a step input, create a flow input for it.

### Special Modules
- Preprocessor: Runs before the first step when triggered externally
  - ID: 'preprocessor'
  - Cannot link inputs
  - Only supports script/rawscript steps
- Error handler: Runs when the flow fails
  - ID: 'failure'
  - Can only reference flow_input and error object
  - Error object structure: { message, name, stack, step_id }
  - Only supports script/rawscript steps

Both modules only support a script or rawscript step. You cannot nest modules using forloop/branchone/branchall.

## Resource types
On Windmill, credentials and configuration are stored in resources. Resource types define the format of the resource.
If the user needs a resource as flow input, you should set the property type in the schema to "object" as well as add a key called "format" and set it to "resource-nameofresourcetype" (e.g. "resource-stripe").
If the user wants a specific resource as step input, you should set the step value to a static string in the following format: "$res:path/to/resource".
`

	return {
		role: 'system',
		content
	}
}

export function prepareFlowUserMessage(
	instructions: string,
	flowAndSelectedId?: { flow: ExtendedOpenFlow; selectedId: string }
): ChatCompletionUserMessageParam {
	const flow = flowAndSelectedId?.flow
	const selectedId = flowAndSelectedId?.selectedId

	if (!flow || !selectedId) {
		return {
			role: 'user',
			content: `## INSTRUCTIONS:
${instructions}`
		}
	}
	return {
		role: 'user',
		content: `## FLOW:
flow_input schema:
${JSON.stringify(flow.schema ?? emptySchema())}

flow modules:
${YAML.stringify(flow.value.modules)}

preprocessor module:
${YAML.stringify(flow.value.preprocessor_module)}

failure module:
${YAML.stringify(flow.value.failure_module)}

currently selected step:
${selectedId}

## INSTRUCTIONS:
${instructions}`
	}
}
