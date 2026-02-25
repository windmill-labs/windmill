import { writable, get } from 'svelte/store'
import { workspaceAIClients } from './components/copilot/lib'
import { type AIProviderModel, type AIProvider, WorkspaceService, type AIConfig } from './gen'
import { COPILOT_SESSION_MODEL_SETTING_NAME, COPILOT_SESSION_PROVIDER_SETTING_NAME } from './stores'
import { getLocalSetting } from './utils'

const USER_CUSTOM_PROMPTS_KEY = 'userCustomAIPrompts'

const sessionModel = getLocalSetting(COPILOT_SESSION_MODEL_SETTING_NAME)
const sessionProvider = getLocalSetting(COPILOT_SESSION_PROVIDER_SETTING_NAME)
export const copilotSessionModel = writable<AIProviderModel | undefined>(
	sessionModel && sessionProvider
		? {
				model: sessionModel,
				provider: sessionProvider as AIProvider
			}
		: undefined
)

export const copilotInfo = writable<{
	enabled: boolean
	codeCompletionModel?: AIProviderModel
	defaultModel?: AIProviderModel
	aiModels: AIProviderModel[]
	customPrompts?: Record<string, string>
	maxTokensPerModel?: Record<string, number>
}>({
	enabled: false,
	codeCompletionModel: undefined,
	defaultModel: undefined,
	aiModels: [],
	customPrompts: {},
	maxTokensPerModel: {}
})

export async function loadCopilot(workspace: string) {
	workspaceAIClients.init(workspace)
	try {
		const info = await WorkspaceService.getCopilotInfo({ workspace })
		setCopilotInfo(info)
	} catch (err) {
		setCopilotInfo({})
		console.error('Could not get copilot info', err)
	}
}

export function setCopilotInfo(aiConfig: AIConfig) {
	if (Object.keys(aiConfig.providers ?? {}).length > 0) {
		const aiModels = Object.entries(aiConfig.providers ?? {}).flatMap(
			([provider, providerConfig]) =>
				providerConfig.models.map((m) => ({ model: m, provider: provider as AIProvider }))
		)

		copilotSessionModel.update((model) => {
			if (
				model &&
				!aiModels.some((m) => m.model === model.model && m.provider === model.provider)
			) {
				return undefined
			}
			return model
		})

		copilotInfo.set({
			enabled: true,
			codeCompletionModel: aiConfig.code_completion_model,
			defaultModel: aiConfig.default_model,
			aiModels: aiModels,
			customPrompts: aiConfig.custom_prompts ?? {},
			maxTokensPerModel: aiConfig.max_tokens_per_model ?? {}
		})
	} else {
		copilotSessionModel.set(undefined)

		copilotInfo.set({
			enabled: false,
			codeCompletionModel: undefined,
			defaultModel: undefined,
			aiModels: [],
			customPrompts: {},
			maxTokensPerModel: {}
		})
	}
}

export function getCurrentModel(): AIProviderModel {
	const model =
		get(copilotSessionModel) ?? get(copilotInfo).defaultModel ?? get(copilotInfo).aiModels[0]
	if (!model) {
		throw new Error('No model selected')
	}
	return model
}

export function tryGetCurrentModel(): AIProviderModel | undefined {
	return get(copilotSessionModel) ?? get(copilotInfo).defaultModel ?? get(copilotInfo).aiModels[0]
}

export function getUserCustomPrompts(): Record<string, string> {
	const stored = getLocalSetting(USER_CUSTOM_PROMPTS_KEY)
	if (stored) {
		try {
			return JSON.parse(stored)
		} catch (e) {
			console.error('Failed to parse user custom prompts', e)
			return {}
		}
	}
	return {}
}

export function getCombinedCustomPrompt(mode: string): string | undefined {
	const workspacePrompt = get(copilotInfo).customPrompts?.[mode]
	const userPrompts = getUserCustomPrompts()
	const userPrompt = userPrompts[mode]

	const prompts = [workspacePrompt, userPrompt].filter((p) => p?.trim())

	if (prompts.length === 0) {
		return undefined
	}

	return prompts.join('\n\n')
}
