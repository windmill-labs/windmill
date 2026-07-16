import { writable, get } from 'svelte/store'
import { workspaceAIClients } from './components/copilot/lib'
import {
	type AIProviderModel,
	type AIProvider,
	WorkspaceService,
	type AIConfig,
	type FreeTierInfo
} from './gen'
import {
	aiUserDisabled,
	COPILOT_SESSION_MODEL_SETTING_NAME,
	COPILOT_SESSION_PROVIDER_SETTING_NAME,
	COPILOT_SESSION_REASONING_SETTING_NAME
} from './stores'
import { getLocalSetting, storeLocalSetting } from './utils'
import {
	type ReasoningProviderModel,
	stripLegacyThinkingSuffix
} from './components/copilot/reasoningRegistry'

const USER_CUSTOM_PROMPTS_KEY = 'userCustomAIPrompts'

const sessionModel = getLocalSetting(COPILOT_SESSION_MODEL_SETTING_NAME)
const sessionProvider = getLocalSetting(COPILOT_SESSION_PROVIDER_SETTING_NAME)
const sessionReasoning = getLocalSetting(COPILOT_SESSION_REASONING_SETTING_NAME)
export const copilotSessionModel = writable<ReasoningProviderModel | undefined>(
	sessionModel && sessionProvider
		? {
				// Strip the deprecated /thinking suffix on read; the default effort
				// (resolved later) restores reasoning for migrated selections.
				model: stripLegacyThinkingSuffix(sessionModel),
				provider: sessionProvider as AIProvider,
				...(sessionReasoning ? { reasoning: sessionReasoning } : {})
			}
		: undefined
)

export const copilotInfo = writable<{
	enabled: boolean
	codeCompletionModel?: AIProviderModel
	defaultModel?: AIProviderModel
	metadataModel?: AIProviderModel
	aiModels: AIProviderModel[]
	customPrompts?: Record<string, string>
	maxTokensPerModel?: Record<string, number>
	webSearchEnabledProviders?: Partial<Record<AIProvider, boolean>>
	// Set only when the workspace has no AI provider of its own and is running on
	// Windmill's free tier. `exhausted` means the grant is spent: there is no model, but
	// that is a different state from "never configured" and the UI must say so.
	freeTier?: FreeTierInfo
}>({
	enabled: false,
	codeCompletionModel: undefined,
	defaultModel: undefined,
	metadataModel: undefined,
	aiModels: [],
	customPrompts: {},
	maxTokensPerModel: {},
	webSearchEnabledProviders: {}
})

// Apply the per-user opt-out live: toggling it flips `enabled` without re-fetching.
// Only enable when providers exist (aiModels is populated whenever the config has any).
aiUserDisabled.subscribe((disabled) => {
	copilotInfo.update((info) => ({
		...info,
		enabled: info.aiModels.length > 0 && !disabled
	}))
})

/** Strip the deprecated /thinking suffix from a configured model slot, if present. */
function stripModelSuffix(model: AIProviderModel | undefined): AIProviderModel | undefined {
	return model ? { ...model, model: stripLegacyThinkingSuffix(model.model) } : model
}

/** Dedupe model entries by provider+model (legacy /thinking entries collapse onto the plain model). */
function dedupeModels(models: AIProviderModel[]): AIProviderModel[] {
	const seen = new Set<string>()
	return models.filter((m) => {
		const key = `${m.provider}:${m.model}`
		if (seen.has(key)) {
			return false
		}
		seen.add(key)
		return true
	})
}

// copilotInfo/copilotSessionModel are global, so concurrent loads (e.g. a fast
// session switch between workspaces) race: an earlier call resolving last would
// clobber the active workspace's config. Apply only the most recent call's
// result via a monotonic token — last invocation wins regardless of resolution
// order. init() is synchronous so its ordering already matches.
let loadCopilotToken = 0
// The workspace copilotInfo currently reflects. A session send awaits this
// matching its committed workspace so getCurrentModel() can't read the previous
// workspace's provider/model while the scoped load is still in flight.
export const copilotWorkspace = writable<string | undefined>(undefined)
// The workspace of the most recent loadCopilot *request*, set synchronously before the
// await — as opposed to `copilotWorkspace`, which only updates once a load resolves. A
// background refresh must compare against this so it can't supersede an in-flight load for
// a newer workspace (which would otherwise win the monotonic token and restore stale state).
export const copilotWorkspaceRequested = writable<string | undefined>(undefined)
export async function loadCopilot(workspace: string) {
	const token = ++loadCopilotToken
	copilotWorkspaceRequested.set(workspace)
	workspaceAIClients.init(workspace)
	try {
		const info = await WorkspaceService.getCopilotInfo({ workspace })
		if (token !== loadCopilotToken) return
		setCopilotInfo(info)
		copilotWorkspace.set(workspace)
	} catch (err) {
		if (token !== loadCopilotToken) return
		setCopilotInfo({})
		copilotWorkspace.set(workspace)
		console.error('Could not get copilot info', err)
	}
}

export function setCopilotInfo(aiConfig: AIConfig) {
	if (Object.keys(aiConfig.providers ?? {}).length > 0) {
		const aiModels = dedupeModels(
			Object.entries(aiConfig.providers ?? {}).flatMap(([provider, providerConfig]) =>
				providerConfig.models.map((m) => ({
					// Strip the deprecated /thinking suffix from workspace-configured models.
					model: stripLegacyThinkingSuffix(m),
					provider: provider as AIProvider
				}))
			)
		)
		const webSearchEnabledProviders = Object.fromEntries(
			Object.entries(aiConfig.providers ?? {}).map(([provider, providerConfig]) => [
				provider,
				providerConfig.web_search_enabled !== false
			])
		) as Partial<Record<AIProvider, boolean>>

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
			// Providers are configured; the per-user opt-out is the only thing that can gate it off.
			enabled: !get(aiUserDisabled),
			// Strip the deprecated /thinking suffix from the configured model slots too,
			// otherwise a workspace whose default still carries it sends an invalid model id.
			codeCompletionModel: stripModelSuffix(aiConfig.code_completion_model),
			defaultModel: stripModelSuffix(aiConfig.default_model),
			metadataModel: stripModelSuffix(aiConfig.metadata_model),
			aiModels: aiModels,
			customPrompts: aiConfig.custom_prompts ?? {},
			maxTokensPerModel: aiConfig.max_tokens_per_model ?? {},
			webSearchEnabledProviders,
			freeTier: aiConfig.free_tier
		})
	} else {
		copilotSessionModel.set(undefined)

		copilotInfo.set({
			enabled: false,
			codeCompletionModel: undefined,
			defaultModel: undefined,
			metadataModel: undefined,
			aiModels: [],
			customPrompts: {},
			maxTokensPerModel: {},
			webSearchEnabledProviders: {},
			// An exhausted free grant lands here — no providers, but the reason AI is off
			// is "you used it up", not "you never set it up".
			freeTier: aiConfig.free_tier
		})
	}
}

export function isWebSearchEnabledForProvider(provider: AIProvider | undefined): boolean {
	if (!provider) {
		return false
	}
	return get(copilotInfo).webSearchEnabledProviders?.[provider] ?? true
}

export function getCurrentModel(): ReasoningProviderModel {
	const model =
		get(copilotSessionModel) ?? get(copilotInfo).defaultModel ?? get(copilotInfo).aiModels[0]
	if (!model) {
		throw new Error('No model selected')
	}
	return model
}

export function getMetadataModel(): AIProviderModel {
	const info = get(copilotInfo)
	const model = info.metadataModel ?? info.defaultModel ?? info.aiModels[0]
	if (!model) {
		throw new Error('No model selected')
	}
	return model
}

export function tryGetCurrentModel(): ReasoningProviderModel | undefined {
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

export function setUserCustomPrompts(prompts: Record<string, string>) {
	storeLocalSetting(USER_CUSTOM_PROMPTS_KEY, JSON.stringify(prompts))
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

// Like getCombinedCustomPrompt but keeps the workspace and user slices separate so the
// Global system prompt can label them distinctly — only the user slice is editable by the
// update_user_instructions tool.
export function getCustomPromptParts(mode: string): { workspace?: string; user?: string } {
	const workspace = get(copilotInfo).customPrompts?.[mode]?.trim() || undefined
	const user = getUserCustomPrompts()[mode]?.trim() || undefined
	return { workspace, user }
}
