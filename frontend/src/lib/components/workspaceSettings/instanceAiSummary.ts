import type { AIProvider, AIProviderModel, GetCopilotInfoResponse } from '$lib/gen'

export type InstanceAIProviderSummary = {
	provider: AIProvider
	models: string[]
}

export type InstanceAISummary = {
	providers: InstanceAIProviderSummary[]
	default_model?: AIProviderModel
	code_completion_model?: AIProviderModel
}

export type GetCopilotInfoResponseWithInstanceAISummary = GetCopilotInfoResponse & {
	instance_ai_summary?: InstanceAISummary
}
