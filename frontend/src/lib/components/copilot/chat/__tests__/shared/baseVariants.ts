import type { ChatCompletionSystemMessageParam } from 'openai/resources/chat/completions.mjs'
import type { ChatCompletionTool } from 'openai/resources/chat/completions.mjs'
import type { VariantConfig } from './types'

/**
 * Generic tool interface that matches the structure used across chat modules.
 */
export interface Tool<THelpers> {
	def: ChatCompletionTool
	fn: (params: {
		args: Record<string, unknown>
		workspace: string
		helpers: THelpers
		toolCallbacks: { setToolStatus: (...args: unknown[]) => void; removeToolStatus: (...args: unknown[]) => void }
		toolId: string
	}) => Promise<string>
}

/**
 * Domain-specific defaults for variant resolution.
 */
export interface VariantDefaults<THelpers> {
	/** Function to prepare system message, optionally with custom prompt */
	prepareSystemMessage: (customPrompt?: string) => ChatCompletionSystemMessageParam
	/** Available tools for the domain */
	tools: Tool<THelpers>[]
}

/**
 * Resolves system prompt from variant config.
 * Returns the appropriate ChatCompletionSystemMessageParam based on config.
 */
export function resolveSystemPrompt<THelpers>(
	variant: VariantConfig | undefined,
	defaults: VariantDefaults<THelpers>,
	fallbackCustomPrompt?: string
): ChatCompletionSystemMessageParam {
	if (!variant?.systemPrompt || variant.systemPrompt.type === 'default') {
		return defaults.prepareSystemMessage(fallbackCustomPrompt)
	}

	if (variant.systemPrompt.type === 'default-with-custom') {
		return defaults.prepareSystemMessage(variant.systemPrompt.custom)
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
export function resolveTools<THelpers>(
	variant: VariantConfig | undefined,
	defaults: VariantDefaults<THelpers>
): {
	toolDefs: ChatCompletionTool[]
	tools: Tool<THelpers>[]
} {
	if (!variant?.tools || variant.tools.type === 'default') {
		return {
			toolDefs: defaults.tools.map((t) => t.def),
			tools: defaults.tools
		}
	}

	if (variant.tools.type === 'subset') {
		const includeList = (variant.tools as { type: 'subset'; include: string[] }).include
		const subset = defaults.tools.filter((t) => includeList.includes(t.def.function.name))
		return {
			toolDefs: subset.map((t) => t.def),
			tools: subset
		}
	}

	if (variant.tools.type === 'custom') {
		// Custom tools are typed as unknown[] in base VariantConfig but domain-specific
		// code should ensure they are the correct Tool<THelpers> type
		const customTools = variant.tools.tools as Tool<THelpers>[]
		return {
			toolDefs: customTools.map((t) => t.def),
			tools: customTools
		}
	}

	// Default fallback
	return {
		toolDefs: defaults.tools.map((t) => t.def),
		tools: defaults.tools
	}
}

/**
 * Resolves model from variant config with fallback.
 */
export function resolveModel(variant?: VariantConfig, fallback?: string): string {
	return variant?.model ?? fallback ?? 'gpt-4o'
}
