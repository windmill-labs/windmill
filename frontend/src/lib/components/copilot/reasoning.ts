import type { AIProvider } from '$lib/gen'

export type AIReasoningEffort = 'none' | 'minimal' | 'low' | 'medium' | 'high' | 'xhigh'

export const AI_REASONING_EFFORTS = [
	'none',
	'minimal',
	'low',
	'medium',
	'high',
	'xhigh'
] as const satisfies readonly AIReasoningEffort[]

export type ReasoningCapability = {
	supported: boolean
	efforts: readonly AIReasoningEffort[]
	defaultEffort?: AIReasoningEffort
	strategy?: 'codex' | 'openai-responses'
}

const OPENAI_REASONING_EFFORTS = ['none', 'low', 'medium', 'high'] as const

export function getReasoningCapability(provider: AIProvider, model?: string): ReasoningCapability {
	if (provider === 'openai_chatgpt_account') {
		return {
			supported: true,
			efforts: AI_REASONING_EFFORTS,
			defaultEffort: getDefaultCodexReasoningEffort(model),
			strategy: 'codex'
		}
	}

	if ((provider === 'openai' || provider === 'azure_openai') && isOpenAIReasoningModel(model)) {
		return {
			supported: true,
			efforts: OPENAI_REASONING_EFFORTS,
			defaultEffort: 'medium',
			strategy: 'openai-responses'
		}
	}

	return { supported: false, efforts: ['none'] }
}

export function normalizeReasoningEffortForCapability(
	effort: AIReasoningEffort | undefined,
	capability: ReasoningCapability
): AIReasoningEffort | undefined {
	if (!capability.supported) return undefined
	if (!effort) return capability.defaultEffort
	if (capability.efforts.includes(effort)) return effort
	if (effort === 'minimal' && capability.efforts.includes('low')) return 'low'
	if (effort === 'xhigh' && capability.efforts.includes('high')) return 'high'
	return capability.defaultEffort
}

export function buildReasoningPayload(
	provider: AIProvider,
	model: string,
	effort: AIReasoningEffort | undefined
): Record<string, unknown> {
	const capability = getReasoningCapability(provider, model)
	const normalizedEffort = normalizeReasoningEffortForCapability(effort, capability)
	if (!capability.supported || !normalizedEffort || normalizedEffort === 'none') {
		return {}
	}

	const apiEffort =
		normalizedEffort === 'minimal' ? 'low' : normalizedEffort === 'xhigh' ? 'high' : normalizedEffort

	return {
		reasoning: {
			effort: apiEffort,
			...(capability.strategy === 'codex' && /^gpt-5/i.test(model) ? { summary: 'auto' } : {})
		}
	}
}

export function getDefaultCodexReasoningEffort(model?: string): AIReasoningEffort {
	return /^gpt-5\.4(?:$|[-.])/i.test(model ?? '') ? 'none' : 'medium'
}

export function isReasoningEffort(value: string | null | undefined): value is AIReasoningEffort {
	return AI_REASONING_EFFORTS.includes(value as AIReasoningEffort)
}

function isOpenAIReasoningModel(model?: string): boolean {
	return /^(o\d|o\d-|o\d\.|gpt-5|gpt-5-|gpt-5\.)/i.test(model ?? '')
}
