import OpenAI from 'openai'
import type { ChatCompletionMessageParam } from 'openai/resources/chat/completions.mjs'
import { prepareFlowUserMessage } from '../../core'
import { createEvalHelpers } from './evalHelpers'
import type { FlowModule } from '$lib/gen'
import type { ToolCallbacks } from '../../../shared'
import { type VariantConfig, resolveSystemPrompt, resolveTools, resolveModel } from './evalVariants'

export interface ToolCallDetail {
	name: string
	arguments: Record<string, any>
}

export interface EvalResult {
	success: boolean
	modules: FlowModule[]
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
}

export interface EvalOptions {
	initialModules?: FlowModule[]
	initialSchema?: Record<string, any>
	model?: string
	customSystemPrompt?: string
	maxIterations?: number
	variant?: VariantConfig
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
	const { helpers, getModules } = createEvalHelpers(
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

			console.log('response', response)

			// Track token usage
			if (response.usage) {
				totalTokens.prompt += response.usage.prompt_tokens
				totalTokens.completion += response.usage.completion_tokens
				totalTokens.total += response.usage.total_tokens
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

		return {
			success: true,
			modules: getModules(),
			tokenUsage: totalTokens,
			toolCallsCount,
			toolsCalled,
			toolCallDetails,
			iterations,
			variantName
		}
	} catch (err) {
		return {
			success: false,
			modules: getModules(),
			error: err instanceof Error ? err.message : String(err),
			tokenUsage: totalTokens,
			toolCallsCount,
			toolsCalled,
			toolCallDetails,
			iterations,
			variantName
		}
	}
}

/**
 * Validates that actual modules match expected modules by order and type.
 */
export function validateModules(
	actual: FlowModule[],
	expected: Array<{ type: string }>
): { valid: boolean; message: string } {
	if (actual.length !== expected.length) {
		return {
			valid: false,
			message: `Module count mismatch: expected ${expected.length}, got ${actual.length}`
		}
	}

	for (let i = 0; i < expected.length; i++) {
		if (actual[i].value.type !== expected[i].type) {
			return {
				valid: false,
				message: `Module ${i} type mismatch: expected '${expected[i].type}', got '${actual[i].value.type}'`
			}
		}
	}

	return { valid: true, message: 'All modules match expected types' }
}

/**
 * Validates that actual tool calls match expected tool calls by order and name.
 */
export function validateToolCalls(
	actual: string[],
	expected: string[]
): { valid: boolean; message: string } {
	if (actual.length !== expected.length) {
		return {
			valid: false,
			message: `Tool call count mismatch: expected ${expected.length}, got ${actual.length}. Called: [${actual.join(', ')}]`
		}
	}

	for (let i = 0; i < expected.length; i++) {
		if (actual[i] !== expected[i]) {
			return {
				valid: false,
				message: `Tool call ${i} mismatch: expected '${expected[i]}', got '${actual[i]}'`
			}
		}
	}

	return { valid: true, message: 'All tool calls match expected' }
}

/**
 * Formats tool call details for logging/debugging.
 */
export function formatToolCalls(details: ToolCallDetail[]): string {
	return details
		.map((d, i) => `${i + 1}. ${d.name}(${JSON.stringify(d.arguments, null, 2)})`)
		.join('\n')
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

/**
 * Formats comparison results as a table for logging.
 */
export function formatComparisonResults(results: EvalResult[]): string {
	const header = 'Variant\t\tSuccess\tTokens\tTools\tIterations'
	const rows = results.map((r) => {
		const name = r.variantName.padEnd(16)
		return `${name}\t${r.success}\t${r.tokenUsage.total}\t${r.toolsCalled.length}\t${r.iterations}`
	})
	return [header, ...rows].join('\n')
}
