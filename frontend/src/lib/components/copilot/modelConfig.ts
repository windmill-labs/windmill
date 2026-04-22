import type { AIProviderModel } from '$lib/gen'

export function anthropicModelDisallowsSamplingParams(model: string) {
	const normalizedModel = model.toLowerCase().replace(/\/thinking$/, '')
	return (
		normalizedModel === 'claude-opus-4-7' ||
		normalizedModel.startsWith('claude-opus-4-7-') ||
		normalizedModel.startsWith('claude-opus-4-7@')
	)
}

export function getDefaultChatTemperature(modelProvider: AIProviderModel): number | undefined {
	if (
		modelProvider.provider === 'anthropic' &&
		anthropicModelDisallowsSamplingParams(modelProvider.model)
	) {
		return undefined
	}

	return 0
}
