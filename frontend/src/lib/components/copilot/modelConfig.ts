import type { AIProviderModel } from '$lib/gen'

export function modelDisallowsSamplingParams(model: string) {
	const normalizedModel = model.toLowerCase()
	return normalizedModel.includes('claude-opus-4-7')
}

export function getDefaultChatTemperature(modelProvider: AIProviderModel): number | undefined {
	if (modelDisallowsSamplingParams(modelProvider.model)) {
		return undefined
	}

	return 0
}
