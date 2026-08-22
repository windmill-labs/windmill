import { ResourceService, WorkspaceService, type AIConfig, type AIProvider } from '$lib/gen'
import { AI_PROVIDERS, fetchAvailableModels } from '../../lib'
import {
	collectAiAgentProviderRefs,
	isModelId,
	sanitizeModelListing,
	selectAiAgentProviderCandidates,
	type AiAgentProviderCatalog,
	type AiAgentProviderOption
} from './aiAgentProviders'

// Read lazily: evaluating this at module scope makes `AI_PROVIDERS` a load-time requirement
// for everything that reaches this module, the whole global chat included.
const aiResourceTypes = () => Object.keys(AI_PROVIDERS) as AIProvider[]

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

/** Kinds whose endpoint serves ids its listing does not contain, so an unlisted id is never
 * grounds for calling it wrong. OpenRouter appends routing suffixes — `:online`, `:nitro`,
 * `:floor` — to any listed model without listing the combinations, and it carries no `base_url`
 * of its own (the backend supplies it), so `customEndpoint` does not cover it. */
const ALIASING_MODEL_LISTING_KINDS: ReadonlySet<string> = new Set(['openrouter'])

/** Each resource costs one model listing call against the provider. A workspace with a
 * long tail of AI resources would otherwise stall every flow write. */
const MAX_PROVIDER_RESOURCES = 8
const MODELS_TIMEOUT_MS = 10_000

/** The catalog lists models without anyone asking — opening a chat is enough — so the response of
 * an endpoint the workspace may not control is capped before it is parsed. Real listings are a few
 * tens of KB; OpenRouter's, the largest, is well under this. */
const MODELS_MAX_BYTES = 2_000_000

/** Anthropic's model listing pages at 20 and `fetchAvailableModels` ignores `has_more`; the AI
 * proxy routes on the path alone, so no caller can ask for more. A listing that fills that page
 * may be truncated, and only a shorter one is provably the whole set. Providers that return every
 * model in one response are not held to it — several legitimately list far more than 20. */
function SINGLE_PAGE_MODEL_COUNT(kind: AIProvider): number {
	return kind === 'anthropic' ? 20 : Number.POSITIVE_INFINITY
}
const CACHE_TTL_MS = 5 * 60_000

const cache = new Map<string, { at: number; promise: Promise<AiAgentProviderCatalog> }>()

const EMPTY_CATALOG: AiAgentProviderCatalog = { options: [], resourcesAreComplete: false }

/** A reference the catalog does not know is either a resource created since it was built or an
 * invented path. Only the first is worth a rebuild, and nothing distinguishes them up front, so
 * the bypass is rate-limited: a model retrying an invented path re-reads at most this often. */
const BYPASS_MIN_INTERVAL_MS = 30_000
const lastBypassAt = new Map<string, number>()

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
	const bypassedAt = lastBypassAt.get(workspace)
	if (bypassedAt !== undefined && Date.now() - bypassedAt < BYPASS_MIN_INTERVAL_MS) {
		return catalog
	}
	lastBypassAt.set(workspace, Date.now())
	cache.delete(workspace)
	return getAiAgentProviderCatalog(workspace)
}

async function loadCatalog(workspace: string): Promise<AiAgentProviderCatalog> {
	let listedAllResources = true
	const [resources, aiConfig] = await Promise.all([
		ResourceService.listResource({
			workspace,
			resourceType: aiResourceTypes().join(',')
		}).catch((err) => {
			console.error('Could not list AI provider resources', err)
			// The catalog no longer knows the workspace's resources, so nothing may be rejected for
			// being absent. Candidates come only from this list — a resource named by the AI
			// settings is not one, since that config may be the instance's, whose resources live
			// in another workspace and cannot resolve here.
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

	const candidates = selectAiAgentProviderCandidates(
		resources,
		new Set(configuredByPath.keys()),
		aiConfig.default_model?.provider,
		(resourceType) => aiResourceTypes().includes(resourceType as AIProvider)
	) as { kind: AIProvider; resourcePath: string }[]

	const kept = candidates.slice(0, MAX_PROVIDER_RESOURCES)
	if (kept.length < candidates.length) {
		console.warn(
			`Listing models for the first ${MAX_PROVIDER_RESOURCES} of ${candidates.length} AI provider resources of ${workspace}`
		)
	}
	const options = await Promise.all(
		kept.map((candidate) => loadOption(workspace, candidate, configuredByPath))
	)

	// `getCopilotInfo` returns the *effective* config, which may be the instance's, and resource
	// paths are workspace-scoped — so a path match proves nothing on its own. The default is
	// offered only when the resource's own live listing names that model, which settles it
	// whichever config the path came from. Completeness is not part of it: a listing rules an
	// absent id out only when it is whole, but a present id is served either way — and the
	// filtered kinds are never whole, so requiring it would deny them a default forever.
	const defaultModel = aiConfig.default_model
	const defaultResourcePath = defaultModel
		? aiConfig.providers?.[defaultModel.provider]?.resource_path
		: undefined
	const servesDefault =
		defaultModel !== undefined &&
		isModelId(defaultModel.model) &&
		options.some(
			(option) =>
				option.resourcePath === defaultResourcePath &&
				option.modelsAreLive &&
				option.models.ids.includes(defaultModel.model)
		)
	return {
		options,
		resourcesAreComplete: listedAllResources && kept.length === candidates.length,
		defaultModel: servesDefault
			? { kind: defaultModel!.provider, model: defaultModel!.model }
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
			AbortSignal.timeout(MODELS_TIMEOUT_MS),
			MODELS_MAX_BYTES
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
	// Everything known about how short of the truth this response may be, stated once.
	const sourceIsWhole =
		!FILTERED_MODEL_LISTING_KINDS.has(candidate.kind) &&
		!ALIASING_MODEL_LISTING_KINDS.has(candidate.kind) &&
		listed.length < SINGLE_PAGE_MODEL_COUNT(candidate.kind)
	const models = sanitizeModelListing(listed, sourceIsWhole)
	if (models.ids.length > 0) {
		return { ...base, models, modelsAreLive: true }
	}
	// Without a live listing, a model id cannot be ruled out: offer the configured
	// models (then the curated defaults) as a hint, but leave the check off.
	const fallback =
		configuredModels.length > 0
			? configuredModels
			: (AI_PROVIDERS[candidate.kind]?.defaultModels ?? [])
	return { ...base, models: sanitizeModelListing(fallback, false), modelsAreLive: false }
}
