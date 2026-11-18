import type { AIConfig, AIProvider } from '$lib/gen'
import { stripLegacyThinkingSuffix } from '../copilot/reasoningRegistry'

/**
 * The rows of a per-model setting, grouped by provider. Settings are keyed by the id
 * the chat reads them under, and `setCopilotInfo` strips the deprecated `/thinking`
 * suffix before the chat ever sees a model. A row built from the raw config would
 * save a value under a key nothing reads, so it would sit in settings looking applied
 * and never take effect. Stripping can collapse two configured slots onto one model,
 * hence the dedupe.
 */
export function chatModelsByProvider(
	aiProviders: Exclude<AIConfig['providers'], undefined>
): Record<string, Array<{ provider: AIProvider; model: string }>> {
	return Object.fromEntries(
		Object.entries(aiProviders).map(([provider, config]) => [
			provider,
			[...new Set(config.models.map(stripLegacyThinkingSuffix))].map((model) => ({
				provider: provider as AIProvider,
				model
			}))
		])
	)
}
