// Keep this module a leaf: it holds AI model *state* only, and must not import the AI
// client (`components/copilot/lib`) or anything under `components/copilot/chat` — those
// import aiStore back, and such a cycle crashes the app once the bundler splits it across
// chunks (docs/frontend-import-cycles.md; the build fails on the chunk cycle, not on this).
import { writable, get } from 'svelte/store'
import { SvelteMap } from 'svelte/reactivity'
import {
	type AIProviderModel,
	type AIProvider,
	type AIConfig,
	type AIWebSearchConfig,
	type FreeTierInfo,
	type ModelPriceOverride,
	WorkspaceService
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

export type CopilotInfo = {
	enabled: boolean
	// The workspace hid the assistant (`ai_config.copilot_disabled`). `enabled` is then false
	// whatever the providers say, and the AI entry points that nudge "configure AI" when
	// `enabled` is off render nothing at all instead.
	workspaceDisabled: boolean
	codeCompletionModel?: AIProviderModel
	defaultModel?: AIProviderModel
	metadataModel?: AIProviderModel
	aiModels: AIProviderModel[]
	customPrompts?: Record<string, string>
	maxTokensPerModel?: Record<string, number>
	/** Context windows per `provider:model`, overriding the built-in window table. */
	contextWindowPerModel?: Record<string, number>
	/** Negotiated rates per `provider:model`, overriding the built-in price table. */
	modelPricing?: Record<string, ModelPriceOverride>
	webSearchEnabledProviders?: Partial<Record<AIProvider, boolean>>
	/** The workspace's search API, which the chat searches through when the model's provider
	 * does not search on its own (`ai_config.web_search`). */
	webSearchBackend?: AIWebSearchConfig['provider']
	// Set only when the workspace has no AI provider of its own and is running on
	// Windmill's free tier. `exhausted` means the grant is spent: there is no model, but
	// that is a different state from "never configured" and the UI must say so.
	freeTier?: FreeTierInfo
}

export const copilotInfo = writable<CopilotInfo>({
	enabled: false,
	workspaceDisabled: false,
	codeCompletionModel: undefined,
	defaultModel: undefined,
	metadataModel: undefined,
	aiModels: [],
	customPrompts: {},
	maxTokensPerModel: {},
	contextWindowPerModel: {},
	modelPricing: {},
	webSearchEnabledProviders: {}
})

// Each workspace's AI config, read by AI sessions, which send with their own workspace's config
// rather than the tab's. Only `loadWorkspaceCopilotInfo` fills it: a save invalidates instead.
const workspaceCopilotInfos = new SvelteMap<string, CopilotInfo>()
const workspaceLoads = new Map<string, Promise<CopilotInfo>>()
// Bumped by invalidation: a load started before it refetches instead of keeping its answer.
let allWorkspacesVersion = 0
const workspaceVersions = new Map<string, number>()
const versionOf = (workspace: string) =>
	`${allWorkspacesVersion}:${workspaceVersions.get(workspace) ?? 0}`

// Apply the per-user opt-out live: toggling it flips `enabled` without re-fetching.
// Only enable when providers exist (aiModels is populated whenever the config has any).
aiUserDisabled.subscribe((disabled) => {
	const withOptOut = (info: CopilotInfo): CopilotInfo => ({
		...info,
		enabled: info.aiModels.length > 0 && !disabled && !info.workspaceDisabled
	})
	copilotInfo.update(withOptOut)
	for (const [workspace, info] of workspaceCopilotInfos) {
		workspaceCopilotInfos.set(workspace, withOptOut(info))
	}
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

// The workspace copilotInfo currently reflects. A session send awaits this
// matching its committed workspace so getCurrentModel() can't read the previous
// workspace's provider/model while the scoped load is still in flight.
export const copilotWorkspace = writable<string | undefined>(undefined)

function buildCopilotInfo(aiConfig: AIConfig, userDisabled: boolean): CopilotInfo {
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

		const workspaceDisabled = aiConfig.copilot_disabled === true
		return {
			// Providers are configured; only the workspace or per-user opt-outs can gate it off.
			enabled: !workspaceDisabled && !userDisabled,
			workspaceDisabled,
			// Strip the deprecated /thinking suffix from the configured model slots too,
			// otherwise a workspace whose default still carries it sends an invalid model id.
			codeCompletionModel: stripModelSuffix(aiConfig.code_completion_model),
			defaultModel: stripModelSuffix(aiConfig.default_model),
			metadataModel: stripModelSuffix(aiConfig.metadata_model),
			aiModels: aiModels,
			customPrompts: aiConfig.custom_prompts ?? {},
			maxTokensPerModel: aiConfig.max_tokens_per_model ?? {},
			contextWindowPerModel: aiConfig.context_window_per_model ?? {},
			webSearchEnabledProviders,
			webSearchBackend: aiConfig.web_search?.provider,
			modelPricing: aiConfig.model_pricing ?? {},
			freeTier: aiConfig.free_tier
		}
	} else {
		return {
			enabled: false,
			workspaceDisabled: aiConfig.copilot_disabled === true,
			codeCompletionModel: undefined,
			defaultModel: undefined,
			metadataModel: undefined,
			aiModels: [],
			customPrompts: {},
			maxTokensPerModel: {},
			contextWindowPerModel: {},
			webSearchEnabledProviders: {},
			modelPricing: {},
			// An exhausted free grant lands here — no providers, but the reason AI is off
			// is "you used it up", not "you never set it up".
			freeTier: aiConfig.free_tier
		}
	}
}

export function isWebSearchEnabledForProvider(
	provider: AIProvider | undefined,
	info: CopilotInfo = get(copilotInfo)
): boolean {
	if (!provider) {
		return false
	}
	return info.webSearchEnabledProviders?.[provider] ?? true
}

export function getCurrentModel(): ReasoningProviderModel {
	return resolveModelFor(get(copilotInfo))
}

export function resolveModelFor(info: CopilotInfo): ReasoningProviderModel {
	const model = tryResolveModelFor(info)
	if (!model) {
		throw new Error('No model selected')
	}
	return model
}

export function getMetadataModel(): AIProviderModel {
	return resolveMetadataModelFor(get(copilotInfo))
}

export function resolveMetadataModelFor(info: CopilotInfo): AIProviderModel {
	const model = info.metadataModel ?? info.defaultModel ?? info.aiModels[0]
	if (!model) {
		throw new Error('No model selected')
	}
	return model
}

export function tryGetCurrentModel(): ReasoningProviderModel | undefined {
	return tryResolveModelFor(get(copilotInfo))
}

/** `workspace`'s AI config if loaded; otherwise starts loading it and returns undefined. */
export function getWorkspaceCopilotInfo(workspace: string): CopilotInfo | undefined {
	const info = workspaceCopilotInfos.get(workspace)
	if (!info) {
		loadWorkspaceCopilotInfo(workspace).catch((e) =>
			console.error(`Failed to load the AI config of workspace ${workspace}`, e)
		)
	}
	return info
}

/** `workspace`'s AI config for a request made on its behalf, without touching the tab's.
 * Fetched once and kept until invalidated. `fresh` refetches a config that may have changed
 * elsewhere: unlike after a save, the one held stays readable until the new one lands, so what
 * shows it does not blank meanwhile, and a load already in flight is as fresh and is shared. */
export function loadWorkspaceCopilotInfo(workspace: string, fresh = false): Promise<CopilotInfo> {
	const known = workspaceCopilotInfos.get(workspace)
	if (known && !fresh) return Promise.resolve(known)
	let load = workspaceLoads.get(workspace)
	if (!load) {
		const version = versionOf(workspace)
		load = WorkspaceService.getCopilotInfo({ workspace })
			// Settled before the retry below, which would otherwise be handed this same load.
			.finally(() => workspaceLoads.delete(workspace))
			.then((config) => {
				if (version !== versionOf(workspace)) return loadWorkspaceCopilotInfo(workspace)
				const info = buildCopilotInfo(config, get(aiUserDisabled))
				workspaceCopilotInfos.set(workspace, info)
				return info
			})
		workspaceLoads.set(workspace, load)
	}
	return load
}

/** After a save of `workspace`'s AI config, or with none of a config every workspace inherits
 * (the instance's): the next request refetches. */
export function invalidateWorkspaceCopilotInfo(workspace?: string) {
	if (workspace) {
		workspaceVersions.set(workspace, (workspaceVersions.get(workspace) ?? 0) + 1)
		workspaceCopilotInfos.delete(workspace)
	} else {
		allWorkspacesVersion++
		workspaceCopilotInfos.clear()
	}
}

export function setCopilotInfo(aiConfig: AIConfig) {
	applyCopilotInfo(buildCopilotInfo(aiConfig, get(aiUserDisabled)))
}

export function applyCopilotInfo(info: CopilotInfo) {
	copilotSessionModel.update((model) => (model && isOffered(info, model) ? model : undefined))
	copilotInfo.set(info)
}

function isOffered(info: CopilotInfo, model: AIProviderModel): boolean {
	return info.aiModels.some((m) => m.model === model.model && m.provider === model.provider)
}

/** The model a request for `info`'s workspace sends: the shared pick when that workspace offers
 * it, else its default, else its first model. Never one the workspace does not offer. */
export function tryResolveModelFor(info: CopilotInfo): ReasoningProviderModel | undefined {
	const picked = get(copilotSessionModel)
	if (picked && isOffered(info, picked)) return picked
	return info.defaultModel ?? info.aiModels[0]
}

/** A workspace a request is made for and that workspace's AI config, kept together so a request
 * never pairs one workspace's credentials with another's settings. */
export type AiTarget = { workspace: string; info: CopilotInfo }

/** What an AI session sends with, for code that otherwise reads the tab's model and config.
 * Both are undefined while the session's config is loading. */
export type SessionAi = { model: ReasoningProviderModel | undefined; info: CopilotInfo | undefined }

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
export function getCustomPromptParts(
	mode: string,
	customPrompts: Record<string, string> | undefined = get(copilotInfo).customPrompts
): { workspace?: string; user?: string } {
	const workspace = customPrompts?.[mode]?.trim() || undefined
	const user = getUserCustomPrompts()[mode]?.trim() || undefined
	return { workspace, user }
}
