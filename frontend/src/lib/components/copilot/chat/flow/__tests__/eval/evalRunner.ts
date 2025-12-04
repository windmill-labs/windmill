import OpenAI from 'openai'
import type { ChatCompletionMessageParam } from 'openai/resources/chat/completions.mjs'
import { flowTools, prepareFlowSystemMessage, prepareFlowUserMessage } from '../../core'
import { createEvalHelpers } from './evalHelpers'
import type { FlowModule } from '$lib/gen'
import type { ToolCallbacks } from '../../../shared'

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
	iterations: number
}

export interface EvalOptions {
	initialModules?: FlowModule[]
	initialSchema?: Record<string, any>
	model?: string
	customSystemPrompt?: string
	maxIterations?: number
}

/**
 * Runs a flow chat evaluation with real OpenAI API calls.
 * Executes tool calls using the actual flowTools from core.ts.
 */
export async function runFlowEval(
	userPrompt: string,
	openaiApiKey: string,
	options?: EvalOptions
): Promise<EvalResult> {
	const client = new OpenAI({ apiKey: openaiApiKey })
	const { helpers, getModules } = createEvalHelpers(
		options?.initialModules ?? [],
		options?.initialSchema
	)

	// Build messages using existing functions from core.ts
	const systemMessage = prepareFlowSystemMessage(options?.customSystemPrompt)
	const userMessage = prepareFlowUserMessage(userPrompt, helpers.getFlowAndSelectedId(), [])

	const messages: ChatCompletionMessageParam[] = [systemMessage, userMessage]
	const toolDefs = flowTools.map((t) => t.def)

	const totalTokens = { prompt: 0, completion: 0, total: 0 }
	let toolCallsCount = 0
	const toolsCalled: string[] = []
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
				model: options?.model ?? 'gpt-4o',
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


			const tool = flowTools.find((t) => t.def.function.name === toolCall.function.name)
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
			iterations
		}
	} catch (err) {
		return {
			success: false,
			modules: getModules(),
			error: err instanceof Error ? err.message : String(err),
			tokenUsage: totalTokens,
			toolCallsCount,
			toolsCalled,
			iterations
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
