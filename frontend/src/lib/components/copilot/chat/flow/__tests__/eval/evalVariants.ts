import type { ChatCompletionSystemMessageParam } from 'openai/resources/chat/completions.mjs'
import type { ChatCompletionTool } from 'openai/resources/chat/completions.mjs'
import { flowTools, prepareFlowSystemMessage } from '../../core'
import type { Tool } from '../../../shared'
import type { FlowAIChatHelpers } from '../../core'

/**
 * Configuration for a variant in eval testing.
 * Allows customizing system prompt, tools, and model for comparison.
 */
export interface VariantConfig {
	name: string
	description?: string

	/** System prompt configuration */
	systemPrompt?:
		| { type: 'default' }
		| { type: 'default-with-custom'; custom: string }
		| { type: 'custom'; content: string }

	/** Tools configuration */
	tools?:
		| { type: 'default' }
		| { type: 'subset'; include: string[] }
		| { type: 'custom'; tools: Tool<FlowAIChatHelpers>[] }

	/** Model to use (default: 'gpt-4o') */
	model?: string
}

/**
 * Resolves system prompt from variant config.
 * Returns the appropriate ChatCompletionSystemMessageParam based on config.
 */
export function resolveSystemPrompt(
	variant?: VariantConfig,
	fallbackCustomPrompt?: string
): ChatCompletionSystemMessageParam {
	if (!variant?.systemPrompt || variant.systemPrompt.type === 'default') {
		return prepareFlowSystemMessage(fallbackCustomPrompt)
	}

	if (variant.systemPrompt.type === 'default-with-custom') {
		return prepareFlowSystemMessage(variant.systemPrompt.custom)
	}

	// type === 'custom'
	return {
		role: 'system',
		content: variant.systemPrompt.content
	}
}

/**
 * Resolves tools from variant config.
 * Returns both the tool definitions (for API) and full tools (for execution).
 */
export function resolveTools(variant?: VariantConfig): {
	toolDefs: ChatCompletionTool[]
	tools: Tool<FlowAIChatHelpers>[]
} {
	if (!variant?.tools || variant.tools.type === 'default') {
		return {
			toolDefs: flowTools.map((t) => t.def),
			tools: flowTools
		}
	}

	if (variant.tools.type === 'subset') {
		const subset = flowTools.filter(
			(t) =>
				variant.tools!.type === 'subset' &&
				(variant.tools as { type: 'subset'; include: string[] }).include.includes(
					t.def.function.name
				)
		)
		return {
			toolDefs: subset.map((t) => t.def),
			tools: subset
		}
	}

	// type === 'custom'
	const customTools = variant.tools.tools

	return {
		toolDefs: customTools.map((t) => t.def),
		tools: customTools
	}
}

/**
 * Resolves model from variant config with fallback.
 */
export function resolveModel(variant?: VariantConfig, fallback?: string): string {
	return variant?.model ?? fallback ?? 'gpt-4o'
}
