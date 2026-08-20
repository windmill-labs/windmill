import { ResourceService, WorkspaceService, type AIConfig, type AIProvider } from '$lib/gen'
import { AI_PROVIDERS, fetchAvailableModels } from '../../lib'
import {
	collectAiAgentProviderRefs,
	type AiAgentProviderCatalog,
	type AiAgentProviderOption
} from './aiAgentProviders'

const AI_RESOURCE_TYPES = Object.keys(AI_PROVIDERS) as AIProvider[]

/** Kinds whose model listing `fetchAvailableModels` narrows before returning it: OpenAI and Azure
 * OpenAI keep only `gpt-`/`o`/`codex` ids (so a fine-tune never appears), Bedrock keeps text
 * models and inference profiles, and Google AI rewrites each id. For those the list is a
 * shortlist to choose from, never grounds for calling an id wrong. */
const FILTERED_MODEL_LISTING_KINDS: ReadonlySet<string> = new Set([
	'openai',
	'azure_openai',
	'googleai',
	'aws_bedrock'
])

/** Each resource costs one model listing call against the provider. A workspace with a
 * long tail of AI resources would otherwise stall every flow write. */
const MAX_PROVIDER_RESOURCES = 8
const MODELS_TIMEOUT_MS = 10_000
const CACHE_TTL_MS = 5 * 60_000

const cache = new Map<string, { at: number; promise: Promise<AiAgentProviderCatalog> }>()

const EMPTY_CATALOG: AiAgentProviderCatalog = { options: [], resourcesAreComplete: false }

/**
 * AI provider resources of a workspace with the models each one serves, for
 * grounding and validating the provider config of `aiagent` flow modules.
 *
 * Never rejects: on failure it resolves to an empty catalog, which downgrades
 * validation to shape checks rather than blocking the write.
 */
export function getAiAgentProviderCatalog(
	workspace: string | undefined
): Promise<AiAgentProviderCatalog> {
	if (!workspace) {
		return Promise.resolve(EMPTY_CATALOG)
	}
	const cached = cache.get(workspace)
	if (cached && Date.now() - cached.at < CACHE_TTL_MS) {
		return cached.promise
	}
	const promise = loadCatalog(workspace).catch((err) => {
		console.error('Could not load AI provider catalog', err)
		// Not cached: a transient failure must not blind validation for the whole TTL.
		cache.delete(workspace)
		return EMPTY_CATALOG
	})
	cache.set(workspace, { at: Date.now(), promise })
	return promise
}

/**
 * The catalog a set of flow modules should be validated against, or undefined when no AI agent
 * step states its own provider — in which case nothing is fetched at all.
 *
 * A resource the cached catalog does not know is re-read once with the cache bypassed before it
 * can be rejected, so a resource created since the catalog was built (by this chat, or in another
 * tab) is not turned away for the rest of the TTL.
 */
export async function getAiAgentProviderCatalogFor(
	workspace: string | undefined,
	modules: unknown
): Promise<AiAgentProviderCatalog | undefined> {
	const { needsCatalog, resourceRefs } = collectAiAgentProviderRefs(modules)
	if (!needsCatalog) {
		return undefined
	}
	const catalog = await getAiAgentProviderCatalog(workspace)
	const unknown = resourceRefs.some(
		(ref) => !catalog.options.some((option) => option.resourceRef === ref)
	)
	if (!unknown || !catalog.resourcesAreComplete || !workspace) {
		return catalog
	}
	cache.delete(workspace)
	return getAiAgentProviderCatalog(workspace)
}

async function loadCatalog(workspace: string): Promise<AiAgentProviderCatalog> {
	let listedAllResources = true
	const [resources, aiConfig] = await Promise.all([
		ResourceService.listResource({
			workspace,
			resourceType: AI_RESOURCE_TYPES.join(',')
		}).catch((err) => {
			console.error('Could not list AI provider resources', err)
			// Whatever the AI settings name still gets a catalog entry below, but the catalog no
			// longer knows the workspace's resources, so nothing may be rejected for being absent.
			listedAllResources = false
			return []
		}),
		// Read the config for this workspace rather than the copilotInfo store, which
		// tracks the navigated workspace and can lag behind a session's own.
		WorkspaceService.getCopilotInfo({ workspace }).catch(() => ({}) as AIConfig)
	])

	const configuredByPath = new Map<string, { kind: AIProvider; models: string[] }>()
	for (const [kind, providerConfig] of Object.entries(aiConfig.providers ?? {})) {
		configuredByPath.set(providerConfig.resource_path, {
			kind: kind as AIProvider,
			models: providerConfig.models ?? []
		})
	}

	const candidates = resources
		.filter((resource) => AI_RESOURCE_TYPES.includes(resource.resource_type as AIProvider))
		.map((resource) => ({
			kind: resource.resource_type as AIProvider,
			resourcePath: resource.path
		}))
	// A resource selected in the AI settings but not readable through the list (another
	// workspace's shared folder, a permission edge) still belongs in the catalog.
	for (const [resourcePath, configured] of configuredByPath) {
		if (!candidates.some((candidate) => candidate.resourcePath === resourcePath)) {
			candidates.push({ kind: configured.kind, resourcePath })
		}
	}

	const defaultProviderKind = aiConfig.default_model?.provider
	const rank = (candidate: { kind: AIProvider; resourcePath: string }) => {
		if (
			defaultProviderKind &&
			configuredByPath.get(candidate.resourcePath)?.kind === defaultProviderKind
		) {
			return 0
		}
		return configuredByPath.has(candidate.resourcePath) ? 1 : 2
	}
	candidates.sort((a, b) => rank(a) - rank(b) || a.resourcePath.localeCompare(b.resourcePath))

	const kept = candidates.slice(0, MAX_PROVIDER_RESOURCES)
	if (kept.length < candidates.length) {
		console.warn(
			`Listing models for the first ${MAX_PROVIDER_RESOURCES} of ${candidates.length} AI provider resources of ${workspace}`
		)
	}
	const options = await Promise.all(
		kept.map((candidate) => loadOption(workspace, candidate, configuredByPath))
	)

	const defaultModel = aiConfig.default_model
	return {
		options,
		resourcesAreComplete: listedAllResources && kept.length === candidates.length,
		defaultModel:
			defaultModel && options.some((option) => option.kind === defaultModel.provider)
				? { kind: defaultModel.provider, model: defaultModel.model }
				: undefined
	}
}

/** A resource whose requests do not reach the provider's own API: an explicit base URL
 * (a gateway such as LiteLLM, or a self-hosted deployment), a non-standard platform such
 * as Vertex AI, or the `customai` kind, which is a base URL by definition. Such an endpoint
 * names its models as it likes, and may accept aliases its listing does not return. */
async function hasCustomEndpoint(
	workspace: string,
	kind: AIProvider,
	path: string
): Promise<boolean> {
	if (kind === 'customai') {
		return true
	}
	try {
		const value = (await ResourceService.getResourceValue({ workspace, path })) as Record<
			string,
			unknown
		> | null
		// `baseUrl` is the azure_openai spelling of `base_url`. Both default to empty,
		// which means the provider's own API.
		const baseUrl = value?.base_url ?? value?.baseUrl
		const platform = value?.platform
		return (
			(typeof baseUrl === 'string' && baseUrl !== '') ||
			(typeof platform === 'string' && platform !== '' && platform !== 'standard')
		)
	} catch (err) {
		console.error(`Could not read AI resource ${path}`, err)
		// Endpoint unknown: assume a proxy, so that a model id is never wrongly rejected.
		return true
	}
}

async function loadOption(
	workspace: string,
	candidate: { kind: AIProvider; resourcePath: string },
	configuredByPath: Map<string, { kind: AIProvider; models: string[] }>
): Promise<AiAgentProviderOption> {
	const configuredModels = configuredByPath.get(candidate.resourcePath)?.models ?? []
	const [customEndpoint, listed] = await Promise.all([
		hasCustomEndpoint(workspace, candidate.kind, candidate.resourcePath),
		fetchAvailableModels(
			candidate.resourcePath,
			workspace,
			candidate.kind,
			AbortSignal.timeout(MODELS_TIMEOUT_MS)
		).catch((err) => {
			console.error(`Could not list models of AI resource ${candidate.resourcePath}`, err)
			return [] as string[]
		})
	])
	const base = {
		kind: candidate.kind,
		resourcePath: candidate.resourcePath,
		resourceRef: `$res:${candidate.resourcePath}`,
		customEndpoint
	}
	if (listed.length > 0) {
		return {
			...base,
			models: listed,
			modelsAreLive: true,
			modelsRuleOutOthers: !customEndpoint && !FILTERED_MODEL_LISTING_KINDS.has(candidate.kind)
		}
	}
	// Without a live listing, a model id cannot be ruled out: offer the configured
	// models (then the curated defaults) as a hint, but leave the check off.
	const fallback =
		configuredModels.length > 0
			? configuredModels
			: (AI_PROVIDERS[candidate.kind]?.defaultModels ?? [])
	return { ...base, models: fallback, modelsAreLive: false, modelsRuleOutOthers: false }
}
