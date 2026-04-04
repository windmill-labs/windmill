import OpenAI from 'openai'
import Anthropic from '@anthropic-ai/sdk'
import type {
	ChatCompletionMessageParam,
	ChatCompletionSystemMessageParam
} from 'openai/resources/chat/completions.mjs'
import type { AIProvider, AIProviderModel } from '$lib/gen/types.gen'
import type { TokenUsage, ToolCallDetail, EvalRunnerOptions } from './types'
import type { Tool } from './baseVariants'
import { runChatLoop, type ChatClients } from '../../../../../frontend/src/lib/components/copilot/chat/chatLoop'
import type {
	Tool as ProductionTool,
	ToolCallbacks
} from '../../../../../frontend/src/lib/components/copilot/chat/shared'

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
	/** Tool definitions for the LLM API (unused — derived from tools) */
	toolDefs?: unknown
	/** Full tool implementations for execution */
	tools: Tool<THelpers>[]
	/** Domain-specific helpers for tool execution */
	helpers: THelpers
	/** API key for the provider */
	apiKey: string
	/** Function to get the current output state */
	getOutput: () => TOutput
	/** Optional configuration */
	options?: EvalRunnerOptions
}

/**
 * Creates SDK clients for the given provider.
 */
function createEvalClients(provider: AIProvider, apiKey: string): ChatClients {
	if (provider === 'anthropic') {
		return {
			openai: new OpenAI({ apiKey: 'unused' }),
			anthropic: new Anthropic({ apiKey })
		}
	}
	return {
		openai: new OpenAI({ apiKey }),
		anthropic: new Anthropic({ apiKey: 'unused' })
	}
}

/**
 * Resolves model string to AIProviderModel.
 */
function resolveModelProvider(
	model: string,
	provider?: AIProvider
): AIProviderModel {
	if (provider) return { provider, model }
	if (model.startsWith('claude')) return { provider: 'anthropic', model }
	if (model.startsWith('gpt') || model.startsWith('o')) return { provider: 'openai', model }
	return { provider: 'openai', model }
}

/**
 * Runs a generic evaluation using the shared chat loop (same code path as production).
 * Uses streaming via real provider SDKs instead of OpenRouter non-streaming.
 */
export async function runEval<THelpers, TOutput>(
	params: RunEvalParams<THelpers, TOutput>
): Promise<RawEvalResult<TOutput>> {
	const {
		systemMessage,
		userMessage,
		tools,
		helpers,
		apiKey,
		getOutput,
		options
	} = params

	const model = options?.model ?? 'gpt-4o'
	const maxIterations = options?.maxIterations ?? 20
	const workspace = options?.workspace ?? 'test-workspace'
	const provider = options?.provider

	const modelProvider = resolveModelProvider(model, provider)
	const clients = createEvalClients(modelProvider.provider, apiKey)

	const messages: ChatCompletionMessageParam[] = [userMessage]
	let toolCallsCount = 0
	const toolsCalled: string[] = []
	const toolCallDetails: ToolCallDetail[] = []

	// Wrap tools to intercept fn calls for tracking.
	// Cast to ProductionTool since the eval Tool has a narrower toolCallbacks type
	// but the actual callbacks passed at runtime will satisfy both interfaces.
	const wrappedTools = tools.map((tool) => ({
		...tool,
		fn: async (p: any) => {
			toolCallsCount++
			toolsCalled.push(tool.def.function.name)
			try {
				const args =
					typeof p.args === 'string' ? JSON.parse(p.args) : p.args
				toolCallDetails.push({ name: tool.def.function.name, arguments: args })
			} catch {
				toolCallDetails.push({
					name: tool.def.function.name,
					arguments: p.args
				})
			}
			return tool.fn(p)
		}
	})) as ProductionTool<THelpers>[]

	// No-op callbacks for eval
	const callbacks: ToolCallbacks & {
		onNewToken: (token: string) => void
		onMessageEnd: () => void
	} = {
		setToolStatus: () => {},
		removeToolStatus: () => {},
		onNewToken: () => {},
		onMessageEnd: () => {}
	}

	const abortController = new AbortController()

	try {
		const result = await runChatLoop({
			messages,
			systemMessage,
			tools: wrappedTools,
			helpers,
			abortController,
			callbacks,
			modelProvider,
			clients,
			workspace,
			maxIterations,
			skipResponsesApi: modelProvider.provider !== 'openai' && modelProvider.provider !== 'azure_openai'
		})

		return {
			success: true,
			output: getOutput(),
			tokenUsage: { prompt: 0, completion: 0, total: 0 },
			toolCallsCount,
			toolsCalled,
			toolCallDetails,
			iterations: Math.max(1, result.addedMessages.filter((m) => m.role === 'assistant').length),
			messages
		}
	} catch (err) {
		let errorMessage: string
		if (err instanceof Error) {
			errorMessage = err.stack ?? err.message
		} else {
			errorMessage = String(err)
		}

		return {
			success: false,
			output: getOutput(),
			error: errorMessage,
			tokenUsage: { prompt: 0, completion: 0, total: 0 },
			toolCallsCount,
			toolsCalled,
			toolCallDetails,
			iterations: 0,
			messages
		}
	}
}
