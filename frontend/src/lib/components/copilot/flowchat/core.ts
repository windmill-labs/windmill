import { ScriptService, type FlowValue } from '$lib/gen'
import { get } from 'svelte/store'
import type {
	ChatCompletionChunk,
	ChatCompletionMessageParam,
	ChatCompletionMessageToolCall,
	ChatCompletionTool
} from 'openai/resources/chat/completions.mjs'
import YAML from 'yaml'
import { getCompletion } from '../lib'
import { workspaceStore } from '$lib/stores'
import { z } from 'zod'
import type { FunctionParameters } from 'openai/resources/shared.mjs'
import { zodToJsonSchema } from 'zod-to-json-schema'
import uFuzzy from '@leeoniya/ufuzzy'
import { emptyString } from '$lib/utils'
import { getLangContext } from '../chat/core'

export type FlowDisplayMessage =
	| {
			role: 'user' | 'assistant'
			content: string
	  }
	| {
			role: 'tool'
			tool_call_id: string
			content: string
	  }

export interface FlowManipulationHelpers {
	insertStep: (location: InsertLocation, step: NewStep) => Promise<string>
	removeStep: (id: string) => Promise<void>
	getStepInputs: (id: string) => Promise<Record<string, any>>
	setStepInputs: (id: string, inputs: string) => Promise<void>
	getFlowInputsSchema: () => Promise<Record<string, any>>
	setFlowInputsSchema: (inputs: Record<string, any>) => Promise<void>
	selectStep: (id: string) => void
	getStepCode: (id: string) => string
	getFlowModules: () => FlowValue
	setBranchPredicate: (id: string, branchIndex: number, expression: string) => Promise<void>
	addBranch: (id: string) => Promise<void>
	removeBranch: (id: string, branchIndex: number) => Promise<void>
	setForLoopIteratorExpression: (id: string, expression: string) => Promise<void>
}

const searchScriptsSchema = z.object({
	query: z
		.string()
		.describe('The query to search for, e.g. send email, list stripe invoices, etc..')
})

const searchScriptsToolDef = createToolDef(
	searchScriptsSchema,
	'search_scripts',
	'Search for scripts in the workpsace'
)

const searchHubScriptsToolDef = createToolDef(
	searchScriptsSchema,
	'search_hub_scripts',
	'Search for scripts in the hub'
)

const newStepSchema = z.union([
	z
		.object({
			type: z.literal('rawscript'),
			language: z
				.enum(['bun', 'python3'])
				.describe('The language to use for the code, default to bun if none specified')
		})
		.describe('Add a raw script step after the given step id'),
	z
		.object({
			type: z.literal('script'),
			path: z.string().describe('The path of the script to use for the step.')
		})
		.describe('Add a script step after the given step id'),
	z
		.object({
			type: z.literal('forloop')
		})
		.describe('Add a for loop after the given step id'),
	z
		.object({
			type: z.literal('branchall')
		})
		.describe('Add a branch all after the given step id: all branches will be executed'),
	z
		.object({
			type: z.literal('branchone')
		})
		.describe(
			'Add a branch one after the given step id: only the first branch that evaluates to true will be executed'
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
					'The id of the step inside which the new step will be added (branchone or branchall only)'
				),
			branchIndex: z
				.number()
				.describe('The index of the branch inside the forloop step, starting at 0')
		})
		.describe(
			'Add a step at the start of a given branch of the given step (branchone or branchall only)'
		)
])

type InsertLocation = z.infer<typeof insertLocationSchema>

const addStepSchema = z.object({
	location: insertLocationSchema,
	step: newStepSchema
})

const addStepToolDef = createToolDef(
	addStepSchema,
	'add_step',
	'Add a step after the given step id'
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
	'Set the iterator expression for the given forloop step'
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
	'Set the predicates for the given branch, only applicable for branchone branches'
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
	'Remove the branch with the given index from the given step, applicable to branchall and branchone steps'
)

const getFlowModulesSchema = z.object({})
const getFlowModulesToolDef = createToolDef(
	getFlowModulesSchema,
	'get_flow_modules',
	'Get the flow modules'
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
	`Set all inputs for the given step id. You should return a list with one element per row of input names and their values using a JavaScript expression. 
Example: "input1: value1\ninput2: value2".
Flow inputs are accessible as flow_input.property_name. The flow input doesn't have to exist already but make sure to add it to the schema if it doesn't.
Step results are accessible as result.stepid or result.stepid.property_name. 
Inside loops, the iterator value is accessible as flow_input.iter.value.
If you want to return static values, just return them like in javascript (e.g. "hello", true, 3, etc...).`
)
const getFlowInputsSchemaSchema = z.object({})

const getFlowInputsSchemaToolDef = createToolDef(
	getFlowInputsSchemaSchema,
	'get_flow_inputs_schema',
	'Get the current flow inputs schema'
)

const setFlowInputsSchemaSchema = z.object({
	schema: z
		.object({
			properties: z.record(z.string(), z.any()).describe('The properties of the flow inputs'),
			required: z.array(z.string()).describe('The required properties of the flow inputs'),
			type: z.literal('object'),
			$schema: z
				.literal('https://json-schema.org/draft/2020-12/schema')
				.describe('The JSON schema of the flow inputs')
		})
		.describe('The JSON schema of the flow inputs')
})

const setFlowInputsSchemaToolDef = createToolDef(
	setFlowInputsSchemaSchema,
	'set_flow_inputs_schema',
	'Set the flow inputs schema. Overrides the current schema, so make sure to use get_flow_inputs_schema to get the current schema first.'
)

const setCodeSchema = z.object({
	code: z.string().describe('The code to apply')
})

const setCodeToolDef = createToolDef(setCodeSchema, 'set_code', 'Set the code for the current step')

interface ToolHelpers extends FlowManipulationHelpers {
	onToolCall: (id: string, content: string) => void
	onFinishToolCall: (id: string, content: string) => void
	setMode: (mode: 'flow' | 'script') => void
	setCode: (code: string) => void
}

interface Tool {
	def: ChatCompletionTool
	fn: ({
		args,
		workspace,
		helpers
	}: {
		args: any
		workspace: string
		helpers: ToolHelpers
		toolId: string
	}) => Promise<string>
}

const flowTools: Tool[] = [
	{
		def: searchScriptsToolDef,
		fn: async ({ args, workspace, helpers, toolId }) => {
			helpers.onToolCall(toolId, 'Searching workspace scripts...')
			const parsedArgs = searchScriptsSchema.parse(args)
			const scripts = await ScriptService.listScripts({
				workspace
			})
			const uf = new uFuzzy()
			const results = uf.search(
				scripts.map((s) => (emptyString(s.summary) ? s.path : s.summary + ' (' + s.path + ')')),
				parsedArgs.query.trim()
			)
			const scriptResults =
				results[2]?.map((id) => ({
					path: scripts[id].path,
					summary: scripts[id].summary
				})) ?? []
			helpers.onFinishToolCall(
				toolId,
				'Found ' + scriptResults.length + ' relevant scripts in the workspace'
			)
			return JSON.stringify(scriptResults)
		}
	},
	{
		def: searchHubScriptsToolDef,
		fn: async ({ args, helpers, toolId }) => {
			helpers.onToolCall(toolId, 'Searching hub scripts...')
			const parsedArgs = searchScriptsSchema.parse(args)
			const scripts = await ScriptService.queryHubScripts({
				text: parsedArgs.query,
				kind: 'script'
			})
			helpers.onFinishToolCall(toolId, 'Found ' + scripts.length + ' relevant scripts in the hub')
			return JSON.stringify(
				scripts.map((s) => ({
					path: `hub/${s.version_id}/${s.app}/${s.summary.toLowerCase().replaceAll(/\s+/g, '_')}`,
					summary: s.summary
				}))
			)
		}
	},
	{
		def: getFlowModulesToolDef,
		fn: async ({ helpers }) => {
			const modules = helpers.getFlowModules()
			return YAML.stringify(modules)
		}
	},
	{
		def: addStepToolDef,
		fn: async ({ args, helpers, toolId }) => {
			helpers.onToolCall(toolId, `Adding step after ${args.afterId}...`)
			const parsedArgs = addStepSchema.parse(args)
			const id = await helpers.insertStep(parsedArgs.location, parsedArgs.step)
			helpers.selectStep(id)

			helpers.onFinishToolCall(toolId, `Step '${id}' added`)

			if (parsedArgs.step.type === 'rawscript') {
				const langContext = getLangContext(parsedArgs.step.language, {
					allowResourcesFetch: true
				})
				return `Step ${id} added, here is some additional instructions to help you write the code:\n${langContext}`
			} else {
				return `Step ${id} added`
			}
		}
	},
	{
		def: removeStepToolDef,
		fn: async ({ args, helpers, toolId }) => {
			helpers.onToolCall(toolId, `Removing step ${args.id}...`)
			const parsedArgs = removeStepSchema.parse(args)
			await helpers.removeStep(parsedArgs.id)
			helpers.onFinishToolCall(toolId, `Step '${parsedArgs.id}' removed`)
			return `Step '${parsedArgs.id}' removed`
		}
	},
	{
		def: getStepInputsToolDef,
		fn: async ({ args, helpers, toolId }) => {
			helpers.onToolCall(toolId, `Getting step ${args.id} inputs...`)
			const parsedArgs = getStepInputsSchema.parse(args)
			const inputs = await helpers.getStepInputs(parsedArgs.id)
			helpers.onFinishToolCall(toolId, `Step '${parsedArgs.id}' inputs retrieved`)
			return JSON.stringify(inputs)
		}
	},
	{
		def: setStepInputsToolDef,
		fn: async ({ args, helpers, toolId }) => {
			helpers.onToolCall(toolId, `Setting step ${args.id} inputs...`)
			const parsedArgs = setStepInputsSchema.parse(args)
			await helpers.setStepInputs(parsedArgs.id, parsedArgs.inputs)
			helpers.selectStep(parsedArgs.id)
			helpers.onFinishToolCall(toolId, `Step '${parsedArgs.id}' inputs set`)
			return `Step '${parsedArgs.id}' inputs set`
		}
	},
	{
		def: getFlowInputsSchemaToolDef,
		fn: async ({ helpers, toolId }) => {
			helpers.onToolCall(toolId, 'Getting flow inputs schema...')
			const schema = await helpers.getFlowInputsSchema()
			helpers.onFinishToolCall(toolId, 'Flow inputs schema retrieved')
			return JSON.stringify(schema)
		}
	},
	{
		def: setFlowInputsSchemaToolDef,
		fn: async ({ args, helpers, toolId }) => {
			helpers.onToolCall(toolId, 'Setting flow inputs schema...')
			const parsedArgs = setFlowInputsSchemaSchema.parse(args)
			await helpers.setFlowInputsSchema(parsedArgs.schema)
			helpers.selectStep('Input')
			helpers.onFinishToolCall(toolId, 'Flow inputs schema set')
			return `Flow inputs schema set`
		}
	},
	{
		def: setCodeToolDef,
		fn: async ({ args, helpers, toolId }) => {
			const parsedArgs = setCodeSchema.parse(args)
			helpers.onToolCall(toolId, 'Setting step code...')
			helpers.setCode(parsedArgs.code)
			helpers.onFinishToolCall(toolId, 'Step code set')
			return `Step code set`
		}
	},
	{
		def: setBranchPredicateToolDef,
		fn: async ({ args, helpers, toolId }) => {
			const parsedArgs = setBranchPredicateSchema.parse(args)
			await helpers.setBranchPredicate(parsedArgs.id, parsedArgs.branchIndex, parsedArgs.expression)
			helpers.selectStep(parsedArgs.id)
			helpers.onFinishToolCall(
				toolId,
				`Branch ${parsedArgs.branchIndex + 1} of '${parsedArgs.id}' predicate set`
			)
			return `Branch ${parsedArgs.branchIndex} of '${parsedArgs.id}' predicate set`
		}
	},
	{
		def: addBranchToolDef,
		fn: async ({ args, helpers, toolId }) => {
			const parsedArgs = addBranchSchema.parse(args)
			await helpers.addBranch(parsedArgs.id)
			helpers.selectStep(parsedArgs.id)
			helpers.onFinishToolCall(toolId, `Branch added to '${parsedArgs.id}'`)
			return `Branch added to '${parsedArgs.id}'`
		}
	},
	{
		def: removeBranchToolDef,
		fn: async ({ args, helpers, toolId }) => {
			const parsedArgs = removeBranchSchema.parse(args)
			await helpers.removeBranch(parsedArgs.id, parsedArgs.branchIndex)
			helpers.selectStep(parsedArgs.id)
			helpers.onFinishToolCall(
				toolId,
				`Branch ${parsedArgs.branchIndex + 1} of '${parsedArgs.id}' removed`
			)
			return `Branch ${parsedArgs.branchIndex} of '${parsedArgs.id}' removed`
		}
	},
	{
		def: setForLoopIteratorExpressionToolDef,
		fn: async ({ args, helpers, toolId }) => {
			const parsedArgs = setForLoopIteratorExpressionSchema.parse(args)
			await helpers.setForLoopIteratorExpression(parsedArgs.id, parsedArgs.expression)
			helpers.selectStep(parsedArgs.id)
			helpers.onFinishToolCall(toolId, `Forloop '${parsedArgs.id}' iterator expression set`)
			return `Forloop '${parsedArgs.id}' iterator expression set`
		}
	}
]

async function callTool(
	tools: Tool[],
	functionName: string,
	args: any,
	workspace: string,
	flowManipulationHelpers: ToolHelpers,
	toolId: string
): Promise<string> {
	const tool = tools.find((t) => t.def.function.name === functionName)
	if (!tool) {
		throw new Error(`Unknown tool call: ${functionName}`)
	}
	return tool.fn({ args, workspace, helpers: flowManipulationHelpers, toolId })
}

async function processToolCall(
	tools: Tool[],
	toolCall: ChatCompletionMessageToolCall,
	messages: ChatCompletionMessageParam[],
	flowManipulationHelpers: ToolHelpers
) {
	try {
		const args = JSON.parse(toolCall.function.arguments || '{}')
		let result = ''
		try {
			result = await callTool(
				tools,
				toolCall.function.name,
				args,
				get(workspaceStore) ?? '',
				flowManipulationHelpers,
				toolCall.id
			)
		} catch (err) {
			console.error(err)
			result =
				'Error while calling tool, MUST tell the user to check the browser console for more details, and then respond as much as possible to the original request'
		}
		messages.push({
			role: 'tool',
			tool_call_id: toolCall.id,
			content: result
		})
	} catch (err) {
		console.error(err)
	}
}

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

export async function chatRequest(
	messages: ChatCompletionMessageParam[],
	abortController: AbortController,
	callbacks: {
		onNewToken: (token: string) => void
		onMessageEnd: () => void
		onToolCall: (id: string, content: string) => void
		onFinishToolCall: (id: string, content: string) => void
		getMode: () => 'flow' | 'script'
		setMode: (mode: 'flow' | 'script') => void
		setCode: (code: string) => void
	},
	flowManipulationHelpers: FlowManipulationHelpers
) {
	try {
		let completion: any = null
		while (true) {
			const systemMessage = prepareSystemMessage()

			// const { modeSystemMessage, modeTools } =
			// 	callbacks.getMode() === 'flow'
			// 		? {
			// 				modeSystemMessage: prepareSystemMessage(),
			// 				modeTools: flowTools
			// 			}
			// 		: {
			// 				modeSystemMessage: prepareSystemMessageScript(),
			// 				modeTools: scriptTools
			// 			}

			completion = await getCompletion(
				[systemMessage, ...messages],
				abortController,
				flowTools.map((t) => t.def)
			)

			if (completion) {
				const finalToolCalls: Record<number, ChatCompletionChunk.Choice.Delta.ToolCall> = {}

				for await (const chunk of completion) {
					if (!('choices' in chunk && chunk.choices.length > 0 && 'delta' in chunk.choices[0])) {
						continue
					}
					const c = chunk as ChatCompletionChunk
					const delta = c.choices[0].delta.content
					if (delta) {
						callbacks.onNewToken(delta)
					}
					const toolCalls = c.choices[0].delta.tool_calls || []
					for (const toolCall of toolCalls) {
						const { index } = toolCall
						const finalToolCall = finalToolCalls[index]
						if (!finalToolCall) {
							finalToolCalls[index] = toolCall
						} else {
							if (toolCall.function?.arguments) {
								if (!finalToolCall.function) {
									finalToolCall.function = toolCall.function
								} else {
									finalToolCall.function.arguments =
										(finalToolCall.function.arguments ?? '') + toolCall.function.arguments
								}
							}
						}
					}
				}

				callbacks.onMessageEnd()

				const toolCalls = Object.values(finalToolCalls).filter(
					(toolCall) => toolCall.id !== undefined && toolCall.function?.arguments !== undefined
				) as ChatCompletionMessageToolCall[]

				if (toolCalls.length > 0) {
					messages.push({
						role: 'assistant',
						tool_calls: toolCalls.map((t) => ({
							...t,
							function: {
								...t.function,
								arguments: t.function.arguments || '{}'
							}
						}))
					})
					for (const toolCall of toolCalls) {
						await processToolCall(flowTools, toolCall, messages, {
							...flowManipulationHelpers,
							...callbacks
						})
					}
				} else {
					break
				}
			}
		}
		return completion
	} catch (err) {
		if (!abortController.signal.aborted) {
			throw err
		}
	}
}

export function prepareSystemMessage(): {
	role: 'system'
	content: string
} {
	const content = `You are a helpful assitant that creates and edit workflows on the Windmill platform. In each user message, you will get a YAML with the current flow. 
    
Based on the user instructions, add/edit/remove steps and link them together using the provided tools.
You can add a script step in different ways:
- from a script in the workspace
- from a script in the hub
- a raw script: create an empty step and write the code directly

When adding a raw script step, you will get back some special instructions to help you write the code. Make sure to follow them carefully. Also, make sure to display the code to the user and ask for confirmation before setting it.

Once you've added a new step and set the path or written the code, you can use the get_step_inputs tool to get the current/possible inputs for the step and then use the set_step_inputs tool to set the inputs for the step using a JavaScript expression.
`

	return {
		role: 'system',
		content
	}
}

export function prepareUserMessage(instructions: string, flowValue: FlowValue) {
	return `FLOW:
${YAML.stringify(flowValue)}

INSTRUCTIONS:
${instructions}`
}
