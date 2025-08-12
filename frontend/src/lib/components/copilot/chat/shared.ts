import type {
	ChatCompletionMessageParam,
	ChatCompletionMessageToolCall,
	ChatCompletionTool
} from 'openai/resources/chat/completions.mjs'
import { get } from 'svelte/store'
import type { ContextElement } from './context'
import { workspaceStore } from '$lib/stores'
import type { ExtendedOpenFlow } from '$lib/components/flows/types'
import type { FunctionParameters } from 'openai/resources/shared.mjs'
import { zodToJsonSchema } from 'zod-to-json-schema'
import { z } from 'zod'
import { ScriptService, JobService, type CompletedJob } from '$lib/gen'

type BaseDisplayMessage = {
	content: string
	contextElements?: ContextElement[]
	snapshot?: ExtendedOpenFlow
}

export type UserDisplayMessage = BaseDisplayMessage & {
	role: 'user'
	index: number // Used to match index with actual chat messages
	error?: boolean
}

export type ToolDisplayMessage = {
	role: 'tool'
	tool_call_id: string
	content: string
	parameters?: any
	result?: any
	isLoading?: boolean
	error?: string
	needsConfirmation?: boolean
	showDetails?: boolean
}

export type AssistantDisplayMessage = BaseDisplayMessage & {
	role: 'assistant'
}

export type DisplayMessage = UserDisplayMessage | ToolDisplayMessage | AssistantDisplayMessage

async function callTool<T>({
	tools,
	functionName,
	args,
	workspace,
	helpers,
	toolCallbacks,
	toolId
}: {
	tools: Tool<T>[]
	functionName: string
	args: any
	workspace: string
	helpers: T
	toolCallbacks: ToolCallbacks
	toolId: string
}): Promise<string> {
	const tool = tools.find((t) => t.def.function.name === functionName)
	if (!tool) {
		throw new Error(`Unknown tool call: ${functionName}. Probably not in the correct mode, use the change_mode tool to switch to the correct mode.`)
	}
	return tool.fn({ args, workspace, helpers, toolCallbacks, toolId })
}

export async function processToolCall<T>({
	tools,
	toolCall,
	helpers,
	toolCallbacks
}: {
	tools: Tool<T>[]
	toolCall: ChatCompletionMessageToolCall
	helpers: T
	toolCallbacks: ToolCallbacks
}): Promise<ChatCompletionMessageParam> {
	try {
		const args = JSON.parse(toolCall.function.arguments || '{}')
		const tool = tools.find((t) => t.def.function.name === toolCall.function.name)
		
		// Check if tool requires confirmation
		const needsConfirmation = tool?.requiresConfirmation
		
		// Add the tool to the display with appropriate status
		toolCallbacks.setToolStatus(toolCall.id, {
			...(needsConfirmation ? { content: 'Waiting for confirmation...' } : {}),
			parameters: args,
			isLoading: true,
			needsConfirmation: needsConfirmation,
			showDetails: tool?.showDetails
		})
		
		// If confirmation is needed and we have the callback, wait for it
		if (needsConfirmation && toolCallbacks.requestConfirmation) {
			const confirmed = await toolCallbacks.requestConfirmation(toolCall.id)
			
			if (!confirmed) {
				toolCallbacks.setToolStatus(toolCall.id, {
					content: 'Cancelled by user',
					isLoading: false,
					error: 'Tool execution was cancelled by user',
					needsConfirmation: false
				})
				return {
					role: 'tool' as const,
					tool_call_id: toolCall.id,
					content: 'Tool execution was cancelled by user'
				}
			}
			
			// Update status to executing after confirmation
			toolCallbacks.setToolStatus(toolCall.id, {
				isLoading: true,
				needsConfirmation: false
			})
		}
		
		let result = ''
		try {
			result = await callTool({
				tools,
				functionName: toolCall.function.name,
				args,
				workspace: get(workspaceStore) ?? '',
				helpers,
				toolCallbacks,
				toolId: toolCall.id
			})
			toolCallbacks.setToolStatus(toolCall.id, {
				isLoading: false,
			})
		} catch (err) {
			console.error(err)
			toolCallbacks.setToolStatus(toolCall.id, {
				isLoading: false,
				error: 'An error occurred while calling the tool'
			})
			const errorMessage = typeof err === 'string' ? err : 'An error occurred while calling the tool'
			result =
				`Error while calling tool: ${errorMessage}, MUST tell the user to check the browser console for more details, and then respond as much as possible to the original request`
		}
		const toAdd = {
			role: 'tool' as const,
			tool_call_id: toolCall.id,
			content: result
		}
		return toAdd
	} catch (err) {
		console.error(err)
		return {
			role: 'tool' as const,
			tool_call_id: toolCall.id,
			content:
				'Error while calling tool, MUST tell the user to check the browser console for more details, and then respond as much as possible to the original request'
		}
	}
}

export interface Tool<T> {
	def: ChatCompletionTool
	fn: (p: {
		args: any
		workspace: string
		helpers: T
		toolCallbacks: ToolCallbacks
		toolId: string
	}) => Promise<string>
	preAction?: (p: { toolCallbacks: ToolCallbacks; toolId: string }) => void
	requiresConfirmation?: boolean
	showDetails?: boolean
}

export interface ToolCallbacks {
	setToolStatus: (id: string, metadata?: Partial<ToolDisplayMessage>) => void
	requestConfirmation?: (toolId: string) => Promise<boolean>
}

export function createToolDef(
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

const searchHubScriptsSchema = z.object({
	query: z
		.string()
		.describe('The query to search for, e.g. send email, list stripe invoices, etc..')
})

const searchHubScriptsToolDef = createToolDef(
	searchHubScriptsSchema,
	'search_hub_scripts',
	'Search for scripts in the hub'
)

export const createSearchHubScriptsTool = (withContent: boolean = false) => ({
	def: searchHubScriptsToolDef,
	fn: async ({ args, toolId, toolCallbacks }) => {
		toolCallbacks.setToolStatus(toolId, {
			content: 'Searching for hub scripts related to "' + args.query + '"...'
		})
		const parsedArgs = searchHubScriptsSchema.parse(args)
		const scripts = await ScriptService.queryHubScripts({
			text: parsedArgs.query,
			kind: 'script'
		})
		toolCallbacks.setToolStatus(toolId, {
			content: 'Found ' + scripts.length + ' scripts in the hub related to "' + args.query + '"'
		})
		// if withContent, fetch scripts with their content, limit to 3 results
		const results = await Promise.all(
			scripts.slice(0, withContent ? 3 : undefined).map(async (s) => {
				let content = ''
				if (withContent) {
					content = await ScriptService.getHubScriptContentByPath({
						path: `hub/${s.version_id}/${s.app}/${s.summary.toLowerCase().replaceAll(/\s+/g, '_')}`
					})
				}
				return {
					path: `hub/${s.version_id}/${s.app}/${s.summary.toLowerCase().replaceAll(/\s+/g, '_')}`,
					summary: s.summary,
					...(withContent ? { content } : {})
				}
			})
		)
		return JSON.stringify(results)
	}
})

// Constants for result formatting
const MAX_RESULT_LENGTH = 12000
const MAX_LOG_LENGTH = 4000

export interface TestRunConfig {
	jobStarter: () => Promise<string>
	workspace: string
	toolCallbacks: ToolCallbacks
	toolId: string
	startMessage?: string
	contextName: 'script' | 'flow'
}

// Common job polling function
export async function pollJobCompletion(
	jobId: string, 
	workspace: string, 
	toolId: string, 
	toolCallbacks: ToolCallbacks
): Promise<CompletedJob> {
	let attempts = 0
	const maxAttempts = 60
	let job: CompletedJob | null = null
	
	while (attempts < maxAttempts) {
		await new Promise(resolve => setTimeout(resolve, 1000))
		attempts++
		
		try {
			const fetchedJob = await JobService.getJob({
				workspace: workspace,
				id: jobId,
				noLogs: false,
				noCode: true
			})
			
			if (fetchedJob.type === 'CompletedJob') {
				job = fetchedJob
				break
			}
		} catch (error) {
			if (attempts >= maxAttempts) {
				throw error
			}
		}
	}
	
	if (!job) {
		toolCallbacks.setToolStatus(toolId, { 
			content: 'Test timed out',
			error: 'Execution timed out or failed to complete'
		})
		throw new Error('Test execution timed out after 60 seconds')
	}
	
	return job
}

// Helper function to extract error messages from job results
function getErrorMessage(result: unknown): string {
	if (typeof result === 'object' && result !== null && 'error' in result) {
		const error = (result as Record<string, unknown>).error
		if (typeof error === 'object' && error !== null && 'message' in error) {
			const message = (error as Record<string, unknown>).message as string
			if ('stack' in error) {
				return message + '\n' + (error as Record<string, unknown>).stack as string
			}
			return message
		}
		if (typeof error === 'string') {
			return error
		}
	}
	if (typeof result === 'string') {
		return result
	}
	return 'Unknown error'
}

// Main execution function for test runs
export async function executeTestRun(config: TestRunConfig): Promise<string> {
	try {
		config.toolCallbacks.setToolStatus(config.toolId, { 
			content: config.startMessage || `Starting ${config.contextName} test...` 
		})
		
		const jobId = await config.jobStarter()
		
		config.toolCallbacks.setToolStatus(config.toolId, { 
			content: `${config.contextName} test started, waiting for completion...` 
		})
		
		const job = await pollJobCompletion(jobId, config.workspace, config.toolId, config.toolCallbacks)
		
		let resultSummary = formatResult(job)
		resultSummary = resultSummary.slice(0, MAX_RESULT_LENGTH)
		
		config.toolCallbacks.setToolStatus(config.toolId, { 
			content: `${config.contextName} test ${job.success ? 'completed successfully' : 'failed'}`,
			result: resultSummary,
			...(job.success ? {} : { error: getErrorMessage(job.result) })
		})
		
		return resultSummary
		
	} catch (error) {
		const errorMessage = error instanceof Error ? error.message : 'Unknown error occurred'
		config.toolCallbacks.setToolStatus(config.toolId, { 
			content: `${config.contextName} test execution failed`,
			error: errorMessage
		})
		throw new Error(`Failed to execute ${config.contextName} test run: ${errorMessage}`)
	}
}

// Helper function to format the result of a test run
export function formatResult(job: CompletedJob): string {
	const success = job.success
	const duration = job.duration_ms ? `${job.duration_ms}ms` : 'unknown'
	const logs = job.logs || 'No logs available'
	
	let resultSummary = `Test ${success ? 'PASSED' : 'FAILED'} (Duration: ${duration})\n\n`

	// Add step-by-step breakdown if available
	const flowStatus = job.flow_status
	if (flowStatus && flowStatus.modules) {
		resultSummary += 'STEP RESULTS:\n'
		for (const [stepId, stepInfo] of Object.entries(flowStatus.modules)) {
			const stepStatus = stepInfo as Record<string, unknown>
			if (stepStatus) {
				const stepSuccess = stepStatus.type === 'Success'
				resultSummary += `- ${stepId}: ${stepSuccess ? 'SUCCESS' : 'FAILED'}\n`
			}
		}
		resultSummary += '\n'
	}
	
	// Add final result
	const result = job.result
	if (success) {
		resultSummary += 'RESULT:\n'
		resultSummary += typeof result === 'string' ? result : JSON.stringify(result, null, 2)
	} else {
		resultSummary += 'ERROR:\n'
		resultSummary += getErrorMessage(result)
	}

	// Add logs with length limiting
	if (logs && logs.trim()) {
		if (logs.length <= MAX_LOG_LENGTH) {
			resultSummary += '\n\nLOGS:\n' + logs
		} else {
			resultSummary += '\n\nLOGS (truncated):\n' + logs.slice(0, MAX_LOG_LENGTH) + '...'
		}
	}

	return resultSummary
}