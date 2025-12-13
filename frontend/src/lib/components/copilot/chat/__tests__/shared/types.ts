import type { ChatCompletionMessageParam } from 'openai/resources/chat/completions.mjs'

/**
 * Token usage tracking for LLM calls.
 */
export interface TokenUsage {
	prompt: number
	completion: number
	total: number
}

/**
 * Details of a single tool call made during evaluation.
 */
export interface ToolCallDetail {
	name: string
	arguments: Record<string, unknown>
}

/**
 * Result of LLM-based comparison/evaluation.
 */
export interface EvaluationResult {
	success: boolean
	resemblanceScore: number
	statement: string
	missingRequirements?: string[]
	error?: string
}

/**
 * Base evaluation result that can be extended for domain-specific outputs.
 * @template TOutput The domain-specific output type (e.g., flow definition, app files)
 */
export interface BaseEvalResult<TOutput> {
	success: boolean
	output: TOutput
	error?: string
	tokenUsage: TokenUsage
	toolCallsCount: number
	toolsCalled: string[]
	toolCallDetails: ToolCallDetail[]
	iterations: number
	variantName: string
	evaluationResult?: EvaluationResult
	messages: ChatCompletionMessageParam[]
}

/**
 * Base configuration for a variant in eval testing.
 * Allows customizing system prompt, tools, and model for comparison.
 *
 * Note: Domain-specific variants may extend this with custom tool configurations.
 * See flow/flowEvalVariants.ts for an example with custom tools.
 */
export interface VariantConfig {
	name: string
	description?: string

	/** System prompt configuration */
	systemPrompt?:
		| { type: 'default' }
		| { type: 'default-with-custom'; custom: string }
		| { type: 'custom'; content: string }

	/** Tools configuration - basic types supported by shared code */
	tools?:
		| { type: 'default' }
		| { type: 'subset'; include: string[] }
		| { type: 'custom'; tools: unknown[] }

	/** Model to use (default: 'gpt-4o') */
	model?: string
}

/**
 * Options for running an evaluation.
 */
export interface EvalRunnerOptions {
	/** Maximum iterations for tool call loop (default: 20) */
	maxIterations?: number
	/** Model to use for LLM calls */
	model?: string
	/** Workspace ID for tool calls */
	workspace?: string
}

/**
 * No-op tool callbacks for eval testing.
 */
export interface ToolCallbacks {
	setToolStatus: (id: string, status: { content?: string; result?: string; error?: string }) => void
	removeToolStatus: (id: string) => void
}

/**
 * Creates no-op tool callbacks for eval testing.
 */
export function createNoOpToolCallbacks(): ToolCallbacks {
	return {
		setToolStatus: () => {},
		removeToolStatus: () => {}
	}
}
