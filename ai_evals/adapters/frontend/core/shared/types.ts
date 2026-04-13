import type { ChatCompletionMessageParam } from 'openai/resources/chat/completions.mjs'
import type { AIProvider } from '$lib/gen/types.gen'

export interface TokenUsage {
	prompt: number
	completion: number
	total: number
}

export interface ToolCallDetail {
	name: string
	arguments: Record<string, unknown>
}

export interface EvalRunnerOptions {
	maxIterations?: number
	model?: string
	workspace?: string
	provider?: AIProvider
}

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
