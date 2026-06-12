import type { AIProviderModel } from '$lib/gen'

export function modelDisallowsSamplingParams(model: string) {
	const normalizedModel = model.toLowerCase()
	// Strip any provider prefix (e.g. OpenRouter's "openai/o3") so the
	// reasoning-model check matches the bare model id rather than the prefix.
	const baseModel = normalizedModel.split('/').pop() ?? normalizedModel
	// gpt-5+ and o-series reasoning models reject sampling params such as
	// temperature (only the default value is supported), regardless of which
	// provider/gateway routes the request — so this must stay provider-agnostic.
	// The o-series match requires a digit after the "o" (o1/o3/o4-mini) so it
	// does not catch unrelated ids like Mistral's "open-mistral-*" or "optimus-*".
	return (
		normalizedModel.includes('claude-fable-5') ||
		normalizedModel.includes('claude-opus-4-7') ||
		normalizedModel.includes('claude-opus-4-8') ||
		baseModel.startsWith('gpt-5') ||
		/^o\d/.test(baseModel)
	)
}

export function getDefaultChatTemperature(modelProvider: AIProviderModel): number | undefined {
	if (modelDisallowsSamplingParams(modelProvider.model)) {
		return undefined
	}

	return 0
}
