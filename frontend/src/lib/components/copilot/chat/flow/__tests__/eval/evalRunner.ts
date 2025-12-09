import OpenAI, { APIError } from 'openai'
import type { ChatCompletionMessageParam } from 'openai/resources/chat/completions.mjs'
import { prepareFlowUserMessage } from '../../core'
import { createEvalHelpers } from './evalHelpers'
import type { FlowModule } from '$lib/gen'
import type { ExtendedOpenFlow } from '$lib/components/flows/types'
import type { ToolCallbacks } from '../../../shared'
import { type VariantConfig, resolveSystemPrompt, resolveTools, resolveModel } from './evalVariants'
import { evaluateFlowComparison, type EvalComparisonResult } from './evalFlowComparison'

// Re-export for convenience
export type { EvalComparisonResult } from './evalFlowComparison'

export interface ToolCallDetail {
	name: string
	arguments: Record<string, any>
}

export interface EvalResult {
	success: boolean
	flow: ExtendedOpenFlow
	error?: string
	tokenUsage: {
		prompt: number
		completion: number
		total: number
	}
	toolCallsCount: number
	toolsCalled: string[]
	toolCallDetails: ToolCallDetail[]
	iterations: number
	variantName: string
	evaluationResult?: EvalComparisonResult
	messages: ChatCompletionMessageParam[]
}

export interface ExpectedFlow {
	summary?: string
	value: {
		modules: FlowModule[]
	}
	schema?: Record<string, any>
}

export interface EvalOptions {
	initialModules?: FlowModule[]
	initialSchema?: Record<string, any>
	model?: string
	customSystemPrompt?: string
	maxIterations?: number
	variant?: VariantConfig
	expectedFlow?: ExpectedFlow
}

/**
 * Runs a flow chat evaluation with real OpenAI API calls.
 * Executes tool calls using the actual flowTools from core.ts or variant-configured tools.
 */
export async function runFlowEval(
	userPrompt: string,
	openaiApiKey: string,
	options?: EvalOptions
): Promise<EvalResult> {
	const client = new OpenAI({ baseURL: 'https://openrouter.ai/api/v1', apiKey: openaiApiKey })
	const { helpers, getFlow } = createEvalHelpers(
		options?.initialModules ?? [],
		options?.initialSchema
	)

	// Resolve variant configuration
	const variantName = options?.variant?.name ?? 'baseline'
	const systemMessage = resolveSystemPrompt(options?.variant, options?.customSystemPrompt)
	const { toolDefs, tools } = resolveTools(options?.variant)
	const model = resolveModel(options?.variant, options?.model)

	// Build user message
	const userMessage = prepareFlowUserMessage(userPrompt, helpers.getFlowAndSelectedId(), [])

	const messages: ChatCompletionMessageParam[] = [systemMessage, userMessage]
	const totalTokens = { prompt: 0, completion: 0, total: 0 }
	let toolCallsCount = 0
	const toolsCalled: string[] = []
	const toolCallDetails: ToolCallDetail[] = []
	let iterations = 0
	const maxIterations = options?.maxIterations ?? 20

	// No-op tool callbacks for eval
	const toolCallbacks: ToolCallbacks = {
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
						workspace: 'test-workspace',
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

		// Run evaluation if expected flow is provided
		let evaluationResult: EvalComparisonResult | undefined
		if (options?.expectedFlow) {
			const generatedFlow = getFlow()
			evaluationResult = await evaluateFlowComparison(
				generatedFlow,
				options.expectedFlow,
				userPrompt
			)
		}

		return {
			success: true,
			flow: getFlow(),
			tokenUsage: totalTokens,
			toolCallsCount,
			toolsCalled,
			toolCallDetails,
			iterations,
			variantName,
			evaluationResult,
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

		// Still run evaluation on partial content if expected flow is provided
		let evaluationResult: EvalComparisonResult | undefined
		if (options?.expectedFlow) {
			try {
				const generatedFlow = getFlow()
				evaluationResult = await evaluateFlowComparison(
					generatedFlow,
					options.expectedFlow,
					userPrompt
				)
			} catch (evalErr) {
				// If evaluation itself fails, just log it and continue
				console.error('Evaluation failed:', evalErr)
			}
		}

		return {
			success: false,
			flow: getFlow(),
			error: errorMessage,
			tokenUsage: totalTokens,
			toolCallsCount,
			toolsCalled,
			toolCallDetails,
			iterations,
			variantName,
			evaluationResult,
			messages
		}
	}
}

/**
 * Runs the same prompt against multiple variants sequentially for comparison.
 * Returns results in the same order as the input variants.
 */
export async function runVariantComparison(
	userPrompt: string,
	variants: VariantConfig[],
	openaiApiKey: string,
	baseOptions?: Omit<EvalOptions, 'variant'>
): Promise<EvalResult[]> {
	const results: EvalResult[] = await Promise.all(
		variants.map(async (variant) => {
			return await runFlowEval(userPrompt, openaiApiKey, {
				...baseOptions,
				variant
			})
		})
	)
	return results
}
