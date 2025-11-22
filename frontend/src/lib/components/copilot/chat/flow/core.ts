import { ScriptService, type FlowModule, type RawScript, type Script, JobService } from '$lib/gen'
import { emitUiIntent } from './uiIntents'
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
	SUPPORTED_CHAT_SCRIPT_LANGUAGES,
	createDbSchemaTool
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

export type AIModuleAction = 'added' | 'modified' | 'removed' | 'shadowed' | undefined

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
	setForLoopOptions: (
		id: string,
		opts: {
			skip_failures?: boolean | null
			parallel?: boolean | null
			parallelism?: number | null
		}
	) => Promise<void>
	setModuleControlOptions: (
		id: string,
		opts: {
			stop_after_if?: boolean | null
			stop_after_if_expr?: string | null
			skip_if?: boolean | null
			skip_if_expr?: string | null
		}
	) => Promise<void>
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

const setForLoopOptionsSchema = z.object({
	id: z.string().describe('The id of the forloop step to configure'),
	skip_failures: z
		.boolean()
		.nullable()
		.optional()
		.describe('Whether to skip failures in the loop (null to not change)'),
	parallel: z
		.boolean()
		.nullable()
		.optional()
		.describe('Whether to run iterations in parallel (null to not change)'),
	parallelism: z
		.number()
		.int()
		.min(1)
		.nullable()
		.optional()
		.describe('Maximum number of parallel iterations (null to not change)')
})

const setForLoopOptionsToolDef = createToolDef(
	setForLoopOptionsSchema,
	'set_forloop_options',
	'Set advanced options for a forloop step: skip_failures, parallel, and parallelism'
)

const setModuleControlOptionsSchema = z.object({
	id: z.string().describe('The id of the module to configure'),
	stop_after_if: z
		.boolean()
		.nullable()
		.optional()
		.describe('Early stop condition (true to set, false to clear, null to not change)'),
	stop_after_if_expr: z
		.string()
		.nullable()
		.optional()
		.describe(
			'JavaScript expression for early stop condition. Can use `flow_input` or `result`. `result` is the result of the step. `results.<step_id>` is not supported, do not use it. Only used if stop_after_if is true. Example: `flow_input.x > 10` or `result === "failure"`'
		),
	skip_if: z
		.boolean()
		.nullable()
		.optional()
		.describe('Skip condition (true to set, false to clear, null to not change)'),
	skip_if_expr: z
		.string()
		.nullable()
		.optional()
		.describe(
			'JavaScript expression for skip condition. Can use `flow_input` or `results.<step_id>`. Only used if skip_if is true. Example: `flow_input.x > 10` or `results.a === "failure"`'
		)
})

const setModuleControlOptionsToolDef = createToolDef(
	setModuleControlOptionsSchema,
	'set_module_control_options',
	'Set control options for any module: stop_after_if (early stop) and skip_if (conditional skip)'
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
			workspace,
			withoutDescription: true
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
		def: addStepToolDef,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const parsedArgs = addStepSchema.parse(args)
			toolCallbacks.setToolStatus(toolId, {
				content:
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
			})
			const id = await helpers.insertStep(parsedArgs.location, parsedArgs.step)
			helpers.selectStep(id)

			toolCallbacks.setToolStatus(toolId, { content: `Added step '${id}'` })

			return `Step ${id} added. Here is the updated flow, make sure to take it into account when adding another step:\n${YAML.stringify(helpers.getModules())}`
		}
	},
	{
		def: removeStepToolDef,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			toolCallbacks.setToolStatus(toolId, { content: `Removing step ${args.id}...` })
			const parsedArgs = removeStepSchema.parse(args)
			helpers.removeStep(parsedArgs.id)
			toolCallbacks.setToolStatus(toolId, { content: `Removed step '${parsedArgs.id}'` })
			return `Step '${parsedArgs.id}' removed. Here is the updated flow:\n${YAML.stringify(helpers.getModules())}`
		}
	},
	{
		def: getStepInputsToolDef,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			toolCallbacks.setToolStatus(toolId, { content: `Getting step ${args.id} inputs...` })
			const parsedArgs = getStepInputsSchema.parse(args)
			const inputs = await helpers.getStepInputs(parsedArgs.id)
			toolCallbacks.setToolStatus(toolId, { content: `Retrieved step '${parsedArgs.id}' inputs` })
			return YAML.stringify(inputs)
		}
	},
	{
		def: setStepInputsToolDef,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			toolCallbacks.setToolStatus(toolId, { content: `Setting step ${args.id} inputs...` })
			const parsedArgs = setStepInputsSchema.parse(args)
			await helpers.setStepInputs(parsedArgs.id, parsedArgs.inputs)
			helpers.selectStep(parsedArgs.id)
			const inputs = await helpers.getStepInputs(parsedArgs.id)
			toolCallbacks.setToolStatus(toolId, { content: `Set step '${parsedArgs.id}' inputs` })
			return `Step '${parsedArgs.id}' inputs set. New inputs:\n${YAML.stringify(inputs)}`
		},
		preAction: ({ toolCallbacks, toolId }) => {
			toolCallbacks.setToolStatus(toolId, { content: 'Setting step inputs...' })
		}
	},
	{
		def: setFlowInputsSchemaToolDef,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			toolCallbacks.setToolStatus(toolId, { content: 'Setting flow inputs schema...' })
			const parsedArgs = setFlowInputsSchemaSchema.parse(args)
			const schema = JSON.parse(parsedArgs.schema)
			await helpers.setFlowInputsSchema(schema)
			helpers.selectStep('Input')
			const updatedSchema = await helpers.getFlowInputsSchema()
			toolCallbacks.setToolStatus(toolId, { content: 'Set flow inputs schema' })
			return `Flow inputs schema set. New schema:\n${JSON.stringify(updatedSchema)}`
		},
		preAction: ({ toolCallbacks, toolId }) => {
			toolCallbacks.setToolStatus(toolId, { content: 'Setting flow inputs schema...' })
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
		def: setCodeToolDef,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const parsedArgs = setCodeSchema.parse(args)
			toolCallbacks.setToolStatus(toolId, {
				content: `Setting code for step '${parsedArgs.id}'...`
			})
			await helpers.setCode(parsedArgs.id, parsedArgs.code)
			helpers.selectStep(parsedArgs.id)
			toolCallbacks.setToolStatus(toolId, { content: `Set code for step '${parsedArgs.id}'` })
			return `Step code set`
		},
		preAction: ({ toolCallbacks, toolId }) => {
			toolCallbacks.setToolStatus(toolId, { content: 'Setting code for step...' })
		}
	},
	{
		def: setBranchPredicateToolDef,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const parsedArgs = setBranchPredicateSchema.parse(args)
			await helpers.setBranchPredicate(parsedArgs.id, parsedArgs.branchIndex, parsedArgs.expression)
			helpers.selectStep(parsedArgs.id)
			toolCallbacks.setToolStatus(toolId, {
				content: `Set predicate of branch ${parsedArgs.branchIndex + 1} of '${parsedArgs.id}'`
			})
			return `Branch ${parsedArgs.branchIndex} of '${parsedArgs.id}' predicate set`
		}
	},
	{
		def: addBranchToolDef,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const parsedArgs = addBranchSchema.parse(args)
			await helpers.addBranch(parsedArgs.id)
			helpers.selectStep(parsedArgs.id)
			toolCallbacks.setToolStatus(toolId, { content: `Added branch to '${parsedArgs.id}'` })
			return `Branch added to '${parsedArgs.id}'`
		}
	},
	{
		def: removeBranchToolDef,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const parsedArgs = removeBranchSchema.parse(args)
			await helpers.removeBranch(parsedArgs.id, parsedArgs.branchIndex)
			helpers.selectStep(parsedArgs.id)
			toolCallbacks.setToolStatus(toolId, {
				content: `Removed branch ${parsedArgs.branchIndex + 1} of '${parsedArgs.id}'`
			})
			return `Branch ${parsedArgs.branchIndex} of '${parsedArgs.id}' removed`
		}
	},
	{
		def: setForLoopIteratorExpressionToolDef,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const parsedArgs = setForLoopIteratorExpressionSchema.parse(args)
			await helpers.setForLoopIteratorExpression(parsedArgs.id, parsedArgs.expression)
			helpers.selectStep(parsedArgs.id)
			toolCallbacks.setToolStatus(toolId, {
				content: `Set forloop '${parsedArgs.id}' iterator expression`
			})
			return `Forloop '${parsedArgs.id}' iterator expression set`
		}
	},
	{
		def: {
			...setForLoopOptionsToolDef,
			function: { ...setForLoopOptionsToolDef.function, strict: false }
		},
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const parsedArgs = setForLoopOptionsSchema.parse(args)
			await helpers.setForLoopOptions(parsedArgs.id, {
				skip_failures: parsedArgs.skip_failures,
				parallel: parsedArgs.parallel,
				parallelism: parsedArgs.parallelism
			})
			helpers.selectStep(parsedArgs.id)

			const message = `Set forloop '${parsedArgs.id}' options`
			toolCallbacks.setToolStatus(toolId, {
				content: message
			})
			return `${message}: ${JSON.stringify(parsedArgs)}`
		}
	},
	{
		def: {
			...setModuleControlOptionsToolDef,
			function: { ...setModuleControlOptionsToolDef.function, strict: false }
		},
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const parsedArgs = setModuleControlOptionsSchema.parse(args)
			await helpers.setModuleControlOptions(parsedArgs.id, {
				stop_after_if: parsedArgs.stop_after_if,
				stop_after_if_expr: parsedArgs.stop_after_if_expr,
				skip_if: parsedArgs.skip_if,
				skip_if_expr: parsedArgs.skip_if_expr
			})
			helpers.selectStep(parsedArgs.id)

			// Emit UI intent to show early-stop tab when stop_after_if is configured
			const modules = helpers.getModules()
			const module = findModuleById(modules, parsedArgs.id)
			if (!module) {
				throw new Error(`Module with id '${parsedArgs.id}' not found in flow.`)
			}
			const moduleType = module?.value.type
			const hasSpecificComponents = ['forloopflow', 'whileloopflow', 'branchall', 'branchone']
			const prefix = hasSpecificComponents.includes(moduleType) ? `${moduleType}` : 'flow'
			if (typeof parsedArgs.stop_after_if === 'boolean') {
				emitUiIntent({
					kind: 'open_module_tab',
					componentId: `${prefix}-${parsedArgs.id}`,
					tab: 'early-stop'
				})
			}

			if (typeof parsedArgs.skip_if === 'boolean') {
				emitUiIntent({
					kind: 'open_module_tab',
					componentId: `${prefix}-${parsedArgs.id}`,
					tab: 'skip'
				})
			}

			const message = `Set module '${parsedArgs.id}' control options`
			toolCallbacks.setToolStatus(toolId, {
				content: message
			})
			return `${message}: ${JSON.stringify(parsedArgs)}`
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
	}
]

export function prepareFlowSystemMessage(customPrompt?: string): ChatCompletionSystemMessageParam {
	let content = `You are a helpful assistant that creates and edits workflows on the Windmill platform. You're provided with a bunch of tools to help you edit the flow.
Follow the user instructions carefully.
Go step by step, and explain what you're doing as you're doing it.
DO NOT wait for user confirmation before performing an action. Only do it if the user explicitly asks you to wait in their initial instructions.
ALWAYS test your modifications. You have access to the \`test_run_flow\` and \`test_run_step\` tools to test the flow and steps. If you only modified a single step, use the \`test_run_step\` tool to test it. If you modified the flow, use the \`test_run_flow\` tool to test it. If the user cancels the test run, do not try again and wait for the next user instruction.
When testing steps that are sql scripts, the arguments to be passed are { database: $res:<db_resource> }.

## Code Markers in Flow Modules

When viewing flow modules, the code content of rawscript steps may include \`[#START]\` and \`[#END]\` markers:
- These markers indicate specific code sections that need attention
- You MUST only modify the code between these markers when using the \`set_code\` tool
- After modifying the code, remove the markers from your response
- If a question is asked about the code, focus only on the code between the markers
- The markers appear in the YAML representation of flow modules when specific code pieces are selected

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
- For forloop steps: 
  - Set the iterator expression using set_forloop_iterator_expression
  - Set advanced options (parallel, parallelism, skip_failures) using set_forloop_options
- For branchone steps: Set the predicates for each branch using set_branch_predicate
- For branchall steps: No additional setup needed

### Module Control Options
For any module type, you can set control flow options using set_module_control_options:
- **stop_after_if**: Early stop condition - stops the module if expression evaluates to true. Can use "flow_input" or "result". "result" is the result of the step. "results.<step_id>" is not supported, do not use it. Example: "flow_input.x > 10" or "result === "failure""
- **skip_if**: Skip condition - skips the module entirely if expression evaluates to true. Can use "flow_input" or "results.<step_id>". Example: "flow_input.x > 10" or "results.a === "failure""

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
- Break condition (stop_after_if) in for loops: result (contains the result of the last iteration)
- Loop iterator: flow_input.iter.value (inside loops)
- Flow inputs: flow_input.property_name
- Static values: Use JavaScript syntax (e.g., "hello", true, 3)

Note: These variables are only accessible in step inputs, forloop iterator expressions and branch predicates. They must be passed as script arguments using the set_step_inputs tool.

For truly static values in step inputs (those not linked to previous steps or loop iterations), prefer using flow inputs by default unless explicitly specified otherwise. This makes the flow more configurable and reusable. For example, instead of hardcoding an email address in a step input, create a flow input for it.

### For Loop Advanced Options
When configuring for-loop steps, consider these options:
- **parallel: true** - Run iterations in parallel for independent operations (significantly faster for I/O bound tasks)
- **parallelism: N** - Limit concurrent iterations (only applies when parallel=true). Use to prevent overwhelming external APIs
- **skip_failures: true** - Continue processing remaining iterations even if some fail. Failed iterations return error objects as results

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
