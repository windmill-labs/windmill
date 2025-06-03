import { ScriptService, type FlowModule, type RawScript, type Script } from '$lib/gen'
import type { ChatCompletionTool } from 'openai/resources/chat/completions.mjs'
import YAML from 'yaml'
import { z } from 'zod'
import { zodToJsonSchema } from 'zod-to-json-schema'
import type { FunctionParameters } from 'openai/resources/shared.mjs'
import uFuzzy from '@leeoniya/ufuzzy'
import { emptySchema, emptyString } from '$lib/utils'
import {
	getFormattedResourceTypes,
	getLangContext,
	SUPPORTED_CHAT_SCRIPT_LANGUAGES
} from '../script/core'
import type { Tool } from '../shared'
import type { ExtendedOpenFlow } from '$lib/components/flows/types'

export interface FlowAIChatHelpers {
	insertStep: (location: InsertLocation, step: NewStep) => Promise<string>
	removeStep: (id: string) => Promise<void>
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

const searchHubScriptsToolDef = createToolDef(
	searchScriptsSchema,
	'search_hub_scripts',
	'Search for scripts in the hub'
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
			)
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
	{
		def: searchScriptsToolDef,
		fn: async ({ args, workspace, toolId, toolCallbacks }) => {
			toolCallbacks.onToolCall(toolId, 'Searching workspace scripts...')
			const parsedArgs = searchScriptsSchema.parse(args)
			const scriptResults = await workspaceScriptsSearch.search(parsedArgs.query, workspace)
			toolCallbacks.onFinishToolCall(
				toolId,
				'Found ' + scriptResults.length + ' relevant scripts in the workspace'
			)
			return JSON.stringify(scriptResults)
		}
	},
	{
		def: searchHubScriptsToolDef,
		fn: async ({ args, toolId, toolCallbacks }) => {
			toolCallbacks.onToolCall(toolId, 'Searching hub scripts...')
			const parsedArgs = searchScriptsSchema.parse(args)
			const scripts = await ScriptService.queryHubScripts({
				text: parsedArgs.query,
				kind: 'script'
			})
			toolCallbacks.onFinishToolCall(
				toolId,
				'Found ' + scripts.length + ' relevant scripts in the hub'
			)
			return JSON.stringify(
				scripts.map((s) => ({
					path: `hub/${s.version_id}/${s.app}/${s.summary.toLowerCase().replaceAll(/\s+/g, '_')}`,
					summary: s.summary
				}))
			)
		}
	},
	{
		def: addStepToolDef,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const parsedArgs = addStepSchema.parse(args)
			toolCallbacks.onToolCall(
				toolId,
				parsedArgs.location.type === 'after'
					? `Adding a step after, ${parsedArgs.location.afterId}`
					: parsedArgs.location.type === 'start'
						? 'Adding a step at the start'
						: parsedArgs.location.type === 'start_inside_forloop'
							? `Adding a step at the start of the forloop ${parsedArgs.location.inside}`
							: parsedArgs.location.type === 'start_inside_branch'
								? `Adding a step at the start of the branch ${parsedArgs.location.inside} ${parsedArgs.location.branchIndex}`
								: parsedArgs.location.type === 'preprocessor'
									? 'Adding a preprocessor step'
									: parsedArgs.location.type === 'failure'
										? 'Adding a failure step'
										: 'Adding a step'
			)
			const id = await helpers.insertStep(parsedArgs.location, parsedArgs.step)
			helpers.selectStep(id)

			toolCallbacks.onFinishToolCall(toolId, `Added step '${id}'`)

			if (parsedArgs.step.type === 'rawscript') {
				return `Step ${id} added. Here is the updated flow, make sure to take it into account when adding another step:\n${YAML.stringify(helpers.getModules())}`
			} else {
				if (
					parsedArgs.location.type === 'start_inside_branch' ||
					parsedArgs.location.type === 'start_inside_forloop'
				) {
					const parentId = parsedArgs.location.inside
					return `Step ${id} added. Here is the updated subflow of step ${parentId}:\n${YAML.stringify(helpers.getModules(parentId))}`
				} else {
					return `Step ${id} added. Here is the updated flow:\n${YAML.stringify(helpers.getModules())}`
				}
			}
		}
	},
	{
		def: removeStepToolDef,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			toolCallbacks.onToolCall(toolId, `Removing step ${args.id}...`)
			const parsedArgs = removeStepSchema.parse(args)
			await helpers.removeStep(parsedArgs.id)
			toolCallbacks.onFinishToolCall(toolId, `Removed step '${parsedArgs.id}'`)
			return `Step '${parsedArgs.id}' removed. Here is the updated flow:\n${YAML.stringify(helpers.getModules())}`
		}
	},
	{
		def: getStepInputsToolDef,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			toolCallbacks.onToolCall(toolId, `Getting step ${args.id} inputs...`)
			const parsedArgs = getStepInputsSchema.parse(args)
			const inputs = await helpers.getStepInputs(parsedArgs.id)
			toolCallbacks.onFinishToolCall(toolId, `Retrieved step '${parsedArgs.id}' inputs`)
			return YAML.stringify(inputs)
		}
	},
	{
		def: setStepInputsToolDef,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			toolCallbacks.onToolCall(toolId, `Setting step ${args.id} inputs...`)
			const parsedArgs = setStepInputsSchema.parse(args)
			await helpers.setStepInputs(parsedArgs.id, parsedArgs.inputs)
			helpers.selectStep(parsedArgs.id)
			const inputs = await helpers.getStepInputs(parsedArgs.id)
			toolCallbacks.onFinishToolCall(toolId, `Set step '${parsedArgs.id}' inputs`)
			return `Step '${parsedArgs.id}' inputs set. New inputs:\n${YAML.stringify(inputs)}`
		}
	},
	{
		def: setFlowInputsSchemaToolDef,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			toolCallbacks.onToolCall(toolId, 'Setting flow inputs schema...')
			const parsedArgs = setFlowInputsSchemaSchema.parse(args)
			const schema = JSON.parse(parsedArgs.schema)
			await helpers.setFlowInputsSchema(schema)
			helpers.selectStep('Input')
			const updatedSchema = await helpers.getFlowInputsSchema()
			toolCallbacks.onFinishToolCall(toolId, 'Set flow inputs schema')
			return `Flow inputs schema set. New schema:\n${JSON.stringify(updatedSchema)}`
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
			toolCallbacks.onFinishToolCall(
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
			toolCallbacks.onToolCall(toolId, `Setting code for step '${parsedArgs.id}'...`)
			await helpers.setCode(parsedArgs.id, parsedArgs.code)
			helpers.selectStep(parsedArgs.id)
			toolCallbacks.onFinishToolCall(toolId, `Set code for step '${parsedArgs.id}'`)
			return `Step code set`
		}
	},
	{
		def: setBranchPredicateToolDef,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const parsedArgs = setBranchPredicateSchema.parse(args)
			await helpers.setBranchPredicate(parsedArgs.id, parsedArgs.branchIndex, parsedArgs.expression)
			helpers.selectStep(parsedArgs.id)
			toolCallbacks.onFinishToolCall(
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
			toolCallbacks.onFinishToolCall(toolId, `Added branch to '${parsedArgs.id}'`)
			return `Branch added to '${parsedArgs.id}'`
		}
	},
	{
		def: removeBranchToolDef,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const parsedArgs = removeBranchSchema.parse(args)
			await helpers.removeBranch(parsedArgs.id, parsedArgs.branchIndex)
			helpers.selectStep(parsedArgs.id)
			toolCallbacks.onFinishToolCall(
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
			toolCallbacks.onFinishToolCall(toolId, `Set forloop '${parsedArgs.id}' iterator expression`)
			return `Forloop '${parsedArgs.id}' iterator expression set`
		}
	},
	{
		def: resourceTypeToolDef,
		fn: async ({ args, toolId, workspace, toolCallbacks }) => {
			const parsedArgs = resourceTypeToolSchema.parse(args)
			toolCallbacks.onToolCall(toolId, 'Searching resource types for "' + parsedArgs.query + '"...')
			const formattedResourceTypes = await getFormattedResourceTypes(
				parsedArgs.language,
				parsedArgs.query,
				workspace
			)
			toolCallbacks.onFinishToolCall(
				toolId,
				'Retrieved resource types for "' + parsedArgs.query + '"'
			)
			return formattedResourceTypes
		}
	}
]

function createToolDef(
	zodSchema: z.ZodSchema,
	name: string,
	description: string
): ChatCompletionTool {
	const schema = zodToJsonSchema(zodSchema, {
		name,
		target: 'openAi'
	})
	let parameters = schema.definitions![name] as FunctionParameters
	parameters = {
		...parameters,
		required: parameters.required ?? []
	}

	return {
		type: 'function',
		function: {
			strict: true,
			name,
			description,
			parameters
		}
	}
}

export function prepareFlowSystemMessage(): {
	role: 'system'
	content: string
} {
	const content = `You are a helpful assitant that creates and edit workflows on the Windmill platform. You're provided with a a bunch of tools to help you edit the flow.
Follow the user instructions carefully.
Go step by step, and explain what you're doing as you're doing it.
DO NOT wait for user confirmation before performing an action. Only do it if the user explicitly asks you to wait in their initial instructions.
Take note of the following:

## Flow editor

### Script steps and inputs
You can add a script step in different ways:
- from a script in the workspace
- from a script in the hub
- a raw script: create an empty step and write the code directly

When generating code for a raw script step, you should make sure it's in the correct format for Windmill. Format varies depending on the language and on the type of step (preprocessor or not). Use the get_instructions_for_code_generation tool to get instructions to help you write the code. Make sure to follow them carefully. DO NOT tell the user you're using this specific tool, just use it.
Also, make sure to display the code to the user before setting it.

Unless the user explicitly specifies how the script should be created (e.g., from the workspace, from the hub, or as a new one), whenever they ask you to add a script step, you must first check for matching scripts in the workspace and the hub. If no suitable script is found, create a raw script step and write the necessary code directly.

Once you've added a new step and set the path or written the code, you should use the get_step_inputs tool to get the current/possible inputs for the step and set the step input values using the set_step_inputs tool.
Step input keys and types are automatically inferred from the code.
If you use flow_input in the step inputs, make sure to add the missing properties to the schema using the set_flow_inputs_schema tool.

### Branchone/branchall and forloop
You can add branchone/branchall and forloops to the flow. Make sure to set the predicates for the branches (of branchone only) and the iterator expression for the forloops.
You can also add or remove branches. 

### Adding multiple steps
After adding a step, make sure to consider the updated flow when adding another step so that the step is added in the right place.

### Flow inputs schema
You can use the set_flow_inputs_schema tool to set the flow inputs schema.

### JavaScript expressions
For step inputs, forloop iterator expressions and branch predicates, you should use JavaScript expressions.
Here are the variables you can use:
- Step results are accessible as results.stepid or results.stepid.property_name. 
- Inside loops, the iterator value is accessible as flow_input.iter.value.
- Flow inputs are accessible as flow_input.property_name. The flow input doesn't have to exist already but make sure to add it to the schema if it doesn't.
- If you want to use static values, set them like in javascript (e.g. "hello", true, 3, etc...).
Note: these variables are only accessible in step inputs, forloop iterator expressions and branch predicates. They are not directly accessible in the code and should be passed as script arguments using the set_step_inputs tool.

### Special modules
- Preprocessor: Runs before the first step when triggered externally. You cannot link its inputs. It's id is 'preprocessor'
- Error handler: Runs when the flow fails. When linking it's input, you can only refer to flow_input and error (error: { message, name, stack, step_id }). It's id is 'failure'. 

Both modules only support a script or rawscript step. You cannot nest modules using foorloop/branchone/branchall.


## Resource types
On Windmill, credentials and configuration are stored in resources. Resource types define the format of the resource.
If the user needs a resource as flow input, you should the property type in the schema to "object" as well as add a key called "format" and set it to "resource-nameofresourcetype" (e.g. "resource-stripe").
If the user wants a specific resource as step input, you should set the step value to a static string in the following format: "$res:path/to/resource".
`

	return {
		role: 'system',
		content
	}
}

export function prepareFlowUserMessage(
	instructions: string,
	flowAndSelectedId: { flow: ExtendedOpenFlow; selectedId: string }
) {
	const { flow, selectedId } = flowAndSelectedId
	return `## FLOW:
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
