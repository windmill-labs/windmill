import OpenAI, { APIError } from 'openai'
import type { ChatCompletionMessageParam, ChatCompletionSystemMessageParam } from 'openai/resources/chat/completions.mjs'
import type { ChatCompletionTool } from 'openai/resources/chat/completions.mjs'
import type { TokenUsage, ToolCallDetail, EvalRunnerOptions } from './types'
import type { Tool } from './baseVariants'

/**
 * Result from a single eval run (before domain-specific evaluation).
 */
export interface RawEvalResult<TOutput> {
	success: boolean
	output: TOutput
	error?: string
	tokenUsage: TokenUsage
	toolCallsCount: number
	toolsCalled: string[]
	toolCallDetails: ToolCallDetail[]
	iterations: number
	messages: ChatCompletionMessageParam[]
}

/**
 * Parameters for running a base evaluation.
 */
export interface RunEvalParams<THelpers, TOutput> {
	/** The user's prompt/instruction */
	userPrompt: string
	/** System message for the LLM */
	systemMessage: ChatCompletionSystemMessageParam
	/** User message for the LLM */
	userMessage: ChatCompletionMessageParam
	/** Tool definitions for the LLM API */
	toolDefs: ChatCompletionTool[]
	/** Full tool implementations for execution */
	tools: Tool<THelpers>[]
	/** Domain-specific helpers for tool execution */
	helpers: THelpers
	/** API key for OpenRouter */
	apiKey: string
	/** Function to get the current output state */
	getOutput: () => TOutput
	/** Optional configuration */
	options?: EvalRunnerOptions
}

/**
 * Runs a generic evaluation with real LLM API calls.
 * Executes tool calls in a loop until the LLM stops calling tools.
 *
 * This is the core execution loop shared across all chat eval tests.
 */
export async function runEval<THelpers, TOutput>(
	params: RunEvalParams<THelpers, TOutput>
): Promise<RawEvalResult<TOutput>> {
	const {
		systemMessage,
		userMessage,
		toolDefs,
		tools,
		helpers,
		apiKey,
		getOutput,
		options
	} = params

	const client = new OpenAI({ baseURL: 'https://openrouter.ai/api/v1', apiKey })
	const model = options?.model ?? 'gpt-4o'
	const maxIterations = options?.maxIterations ?? 20
	const workspace = options?.workspace ?? 'test-workspace'

	const messages: ChatCompletionMessageParam[] = [systemMessage, userMessage]
	const totalTokens: TokenUsage = { prompt: 0, completion: 0, total: 0 }
	let toolCallsCount = 0
	const toolsCalled: string[] = []
	const toolCallDetails: ToolCallDetail[] = []
	let iterations = 0

	// No-op tool callbacks for eval
	const toolCallbacks = {
		setToolStatus: () => {},
		removeToolStatus: () => {}
	}

	try {
		// Tool resolution loop
		while (iterations < maxIterations) {
			iterations++

			const response = await client.chat.completions.create({
				model,
				messages,
				tools: toolDefs,
				temperature: 0
			})

			// Track token usage
			if (response.usage) {
				totalTokens.prompt += response.usage.prompt_tokens
				totalTokens.completion += response.usage.completion_tokens
				totalTokens.total += response.usage.total_tokens
			}

			if (!response.choices.length) {
				throw new Error('No response from API')
			}

			const choice = response.choices[0]
			const assistantMessage = choice.message

			// Add assistant message to history
			messages.push(assistantMessage)

			// If no tool calls, we're done
			if (!assistantMessage.tool_calls?.length) {
				break
			}

			// Execute each tool call
			for (const toolCall of assistantMessage.tool_calls) {
				toolCallsCount++

				// Type guard: only handle function tool calls
				if (toolCall.type !== 'function') {
					messages.push({
						role: 'tool',
						tool_call_id: toolCall.id,
						content: `Unsupported tool type: ${toolCall.type}`
					})
					continue
				}

				toolsCalled.push(toolCall.function.name)

				const tool = tools.find((t) => t.def.function.name === toolCall.function.name)
				if (!tool) {
					messages.push({
						role: 'tool',
						tool_call_id: toolCall.id,
						content: `Unknown tool: ${toolCall.function.name}`
					})
					continue
				}

				try {
					const args = JSON.parse(toolCall.function.arguments)
					toolCallDetails.push({ name: toolCall.function.name, arguments: args })
					const result = await tool.fn({
						args,
						workspace,
						helpers,
						toolCallbacks,
						toolId: toolCall.id
					})
					messages.push({
						role: 'tool',
						tool_call_id: toolCall.id,
						content: result
					})
				} catch (err) {
					const errorMessage = err instanceof Error ? err.message : String(err)
					messages.push({
						role: 'tool',
						tool_call_id: toolCall.id,
						content: `Error: ${errorMessage}`
					})
				}
			}
		}

		return {
			success: true,
			output: getOutput(),
			tokenUsage: totalTokens,
			toolCallsCount,
			toolsCalled,
			toolCallDetails,
			iterations,
			messages
		}
	} catch (err) {
		// Build detailed error message
		let errorMessage: string
		if (err instanceof APIError) {
			const details: string[] = [`${err.status} ${err.message}`]
			if (err.code) details.push(`Code: ${err.code}`)
			if (err.type) details.push(`Type: ${err.type}`)
			if (err.param) details.push(`Param: ${err.param}`)
			if (err.requestID) details.push(`Request ID: ${err.requestID}`)
			if (err.error && typeof err.error === 'object') {
				details.push(`Response: ${JSON.stringify(err.error, null, 2)}`)
			}
			errorMessage = details.join('\n')
		} else if (err instanceof Error) {
			errorMessage = err.stack ?? err.message
		} else {
			errorMessage = String(err)
		}

		return {
			success: false,
			output: getOutput(),
			error: errorMessage,
			tokenUsage: totalTokens,
			toolCallsCount,
			toolsCalled,
			toolCallDetails,
			iterations,
			messages
		}
	}
}
