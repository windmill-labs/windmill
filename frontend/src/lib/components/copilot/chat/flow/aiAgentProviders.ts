import type { AIProvider } from '$lib/gen'
import { forEachAiAgentModule } from '$lib/components/flows/aiAgentModules'

/** One AI provider resource of a workspace, with the models it serves. */
export type AiAgentProviderOption = {
	kind: AIProvider
	resourcePath: string
	/** `$res:<path>`, the form an AI agent step's provider config references. */
	resourceRef: string
	models: ModelListing
	/** `models` came from the endpoint's own listing. False when that listing failed and the
	 * models are a fallback, which cannot rule an id out. */
	modelsAreLive: boolean
	/** The resource points at a gateway rather than the provider's own API (custom `base_url`,
	 * a non-standard platform, or the `customai` kind). Such an endpoint may accept aliases its
	 * listing omits. Unknown endpoints count as custom, never the other way round. */
	customEndpoint: boolean
}

export type AiAgentProviderCatalog = {
	options: AiAgentProviderOption[]
	/** `options` holds every AI provider resource of the workspace. False when the listing failed
	 * or hit the per-workspace cap, in which case a resource missing from `options` may still be
	 * a real one and must not be rejected. */
	resourcesAreComplete: boolean
	/** The workspace default, when one of `options` is the resource its provider is configured
	 * with and that resource's live listing names the model. Absent otherwise, which puts the
	 * choice back to the user. */
	defaultModel?: { kind: AIProvider; model: string }
}

/** A model id as every provider writes one: `claude-sonnet-5`, `meta-llama/Llama-3.3-70B`,
 * `anthropic.claude-haiku-4-5-20251001-v1:0`, `ft:gpt-4o:acme::abc`. Anything else is not
 * rendered: a resource may point at a gateway someone else controls, and its listing lands in a
 * system prompt, so a value with newlines or backticks could append instructions to the chat's
 * own context. Length-bounded for the same reason. */
const MODEL_ID = /^[A-Za-z0-9][A-Za-z0-9._:+@/-]{0,79}$/

/** Whether a string is usable as a model id at all. */
export function isModelId(value: unknown): value is string {
	return typeof value === 'string' && MODEL_ID.test(value)
}

/**
 * A resource's models, carrying whether they are the whole set.
 *
 * `complete` is the single fact validation may reject an id on, and it is set here rather than
 * derived per call site: every way this list can fall short of what the endpoint serves — a
 * filtered listing, a paginated one, the entry cap below, an unusable entry dropped, or no
 * listing at all — has to flow through `sanitizeModelListing`, so a new truncation cannot quietly
 * leave the list looking exhaustive.
 */
export type ModelListing = { ids: string[]; complete: boolean }

/**
 * Models kept per resource. This is the set membership questions are answered from — is this id
 * served? — so it is deliberately far larger than the {@link MAX_PROMPTED_MODELS} the prompt
 * shows: OpenRouter lists hundreds, and a workspace default among them must still be recognised.
 * The response itself is capped in bytes before it is parsed, so this is a backstop on entries
 * rather than the memory bound; reaching it means the set is no longer whole.
 */
const MAX_MODELS_PER_RESOURCE = 5000

/** Keep the entries of a provider's model listing that are usable as an id, and record whether
 * what is left is still the endpoint's whole set. `sourceIsWhole` is the caller's claim about the
 * response itself: false for a listing the caller filtered, paginated, or did not make. */
export function sanitizeModelListing(
	ids: readonly unknown[],
	sourceIsWhole: boolean
): ModelListing {
	const kept: string[] = []
	let dropped = false
	for (const id of ids) {
		if (kept.length === MAX_MODELS_PER_RESOURCE) {
			dropped = true
			break
		}
		if (isModelId(id)) {
			kept.push(id)
		} else {
			dropped = true
		}
	}
	return { ids: kept, complete: sourceIsWhole && !dropped }
}

/** Models listed per resource in the prompt. Providers expose hundreds of ids (OpenRouter,
 * Bedrock); the whole list would crowd out the flow instructions. */
const MAX_PROMPTED_MODELS = 25

/** An id outside `models.ids` is provably wrong for this resource: the listing is live, whole,
 * and from an endpoint that serves what it lists. Anything short of that can only be reported. */
function rulesOutOtherModels(option: AiAgentProviderOption): boolean {
	return option.modelsAreLive && option.models.complete && !option.customEndpoint
}

function describeOption(option: AiAgentProviderOption): string {
	const models = option.models.ids.slice(0, MAX_PROMPTED_MODELS)
	const more =
		option.models.ids.length > models.length
			? `, ... (${option.models.ids.length - models.length} more)`
			: ''
	const caveat = !option.modelsAreLive
		? ' (the endpoint would not list its models, so these are a guess — confirm the id with the user)'
		: !rulesOutOtherModels(option)
			? ' (the endpoint may also serve model names this list does not show)'
			: ''
	const modelList = models.length > 0 ? `${models.join(', ')}${more}${caveat}` : 'none listed'
	return `- \`${option.resourceRef}\` (kind \`${option.kind}\`) — models: ${modelList}`
}

/**
 * Prompt section naming the AI provider resources an AI agent step may reference and the models
 * each one serves. The provider config's shape is in the flow authoring reference, not repeated
 * here; this is the part that only exists at run time.
 *
 * Empty string only when the catalog does not know the workspace's resources — a workspace that
 * definitively has none still gets a section saying so. Callers append whatever is non-empty.
 */
export function formatAiAgentProvidersPrompt(
	catalog: AiAgentProviderCatalog,
	{ canAskUser }: { canAskUser: boolean }
): string {
	if (catalog.options.length === 0) {
		// "None listed" and "none exist" are different facts, and only the second is worth a line:
		// an AI agent step cannot be written at all until someone creates a provider resource.
		return catalog.resourcesAreComplete
			? `## AI provider resources in this workspace

This workspace has none, so an AI agent step has no model to run on. ${
					canAskUser
						? 'Ask the user to create an AI provider resource (Anthropic, OpenAI, ...) before writing one'
						: 'Tell the user to create an AI provider resource (Anthropic, OpenAI, ...) before writing one'
				}, rather than referencing a resource path that does not exist.`
			: ''
	}
	// One resource plus a workspace default leaves nothing to decide. Anything else — several
	// resources to choose between, or no default model — is the user's call, so it is put to them
	// wherever the chat can ask. A chat without the tool says what it picked instead.
	const choiceLine =
		catalog.options.length === 1 && catalog.defaultModel
			? `Unless the user asks for something else, use kind \`${catalog.defaultModel.kind}\` with model \`${catalog.defaultModel.model}\` — the workspace default. Name the model you used in your reply.`
			: canAskUser
				? `When the user has not said which provider resource or model the agent should use, ask with \`askUserQuestion\` before writing the step, offering the resources and models above as proposed answers. Do not pick one yourself.`
				: `When the user has not said which provider resource or model the agent should use, pick the one that best fits what they asked for and name it in your reply, so they can correct it.`
	const truncationLine = catalog.resourcesAreComplete
		? ''
		: '\nThis list is incomplete: the workspace has AI provider resources that are not shown.'
	return `## AI provider resources in this workspace

An AI agent step's \`model\` must be one of the ids listed below for the resource it references — never a model id from memory, which the endpoint would reject at run time.

${catalog.options.map(describeOption).join('\n')}${truncationLine}
${choiceLine}`
}

/**
 * The workspace resources an AI agent step may reference, in the order the prompt should offer
 * them: the workspace default's provider first, then the rest of what the AI settings configured.
 *
 * Only resources the workspace itself lists are candidates. `getCopilotInfo` falls back to the
 * *instance* AI settings when a workspace configures none, and those resource paths live in the
 * `admins` workspace — a `$res:` reference to one resolves against the flow's own workspace at run
 * time and fails with "Resource not found", so offering it would be worse than offering nothing.
 */
export function selectAiAgentProviderCandidates(
	listed: readonly { path: string; resource_type: string }[],
	configuredPaths: ReadonlySet<string>,
	defaultProviderKind: string | undefined,
	isAiResourceType: (resourceType: string) => boolean
): { kind: string; resourcePath: string }[] {
	const candidates = listed
		.filter((resource) => isAiResourceType(resource.resource_type))
		.map((resource) => ({ kind: resource.resource_type, resourcePath: resource.path }))
	const rank = (candidate: { kind: string; resourcePath: string }) => {
		if (!configuredPaths.has(candidate.resourcePath)) return 2
		return candidate.kind === defaultProviderKind ? 0 : 1
	}
	return candidates.sort((a, b) => rank(a) - rank(b) || a.resourcePath.localeCompare(b.resourcePath))
}

/** What a set of modules needs from the catalog: `needsCatalog` is false when no AI agent step
 * states a provider of its own, and `resourceRefs` are the `$res:` references those steps use. */
export function collectAiAgentProviderRefs(modules: unknown): {
	needsCatalog: boolean
	resourceRefs: string[]
} {
	let needsCatalog = false
	const resourceRefs = new Set<string>()
	forEachAiAgentModule(modules, (_mod, value) => {
		if (value.agent) return
		const transform = value.input_transforms?.provider
		if (!transform || transform.type !== 'static') return
		needsCatalog = true
		const resource = (transform.value as Record<string, unknown> | undefined)?.resource
		if (typeof resource === 'string') {
			resourceRefs.add(resource)
		}
	})
	return { needsCatalog, resourceRefs: [...resourceRefs] }
}

/** Tool-result suffix for provider findings that did not block the write. Empty when there are none. */
export function formatAiAgentProviderWarnings(warnings: string[]): string {
	if (warnings.length === 0) return ''
	return `\n\nAI agent provider warnings:\n${warnings.join('\n')}`
}

function knownOptionsHint(catalog: AiAgentProviderCatalog): string {
	if (catalog.options.length === 0) return ''
	return `\nAI provider resources in this workspace:\n${catalog.options.map(describeOption).join('\n')}`
}

const PROVIDER_SHAPE =
	'{ "type": "static", "value": { "kind": "<provider kind>", "resource": "$res:<resource path>", "model": "<model id>" } }'

/** `settled` marks a finding the catalog established rather than one it could not rule out, so
 * the tool result does not hedge a fact. */
type ProviderIssue = { message: string; blocking: boolean; settled?: boolean }

function blocking(message: string): ProviderIssue {
	return { message, blocking: true }
}

function checkProviderValue(
	value: unknown,
	catalog: AiAgentProviderCatalog
): ProviderIssue | undefined {
	if (typeof value === 'string') {
		return blocking(
			`provider must be an object ${PROVIDER_SHAPE}, not the resource reference string ${JSON.stringify(value)}`
		)
	}
	if (!value || typeof value !== 'object' || Array.isArray(value)) {
		return blocking(`provider must be an object ${PROVIDER_SHAPE}`)
	}
	const { kind, resource, model } = value as Record<string, unknown>
	if (typeof kind !== 'string' || kind === '') {
		return blocking(`provider.kind is missing. Expected ${PROVIDER_SHAPE}`)
	}
	if (typeof resource !== 'string' || !resource.startsWith('$res:')) {
		return blocking(
			`provider.resource must be a "$res:<resource path>" reference. Expected ${PROVIDER_SHAPE}`
		)
	}
	if (typeof model !== 'string' || model === '') {
		return blocking(`provider.model is missing. Expected ${PROVIDER_SHAPE}`)
	}
	if (catalog.options.length === 0) {
		return catalog.resourcesAreComplete
			? {
					message: `this workspace has no AI provider resources, so "${resource}" cannot resolve and the step cannot run. Tell the user to create an AI provider resource (Anthropic, OpenAI, ...) and rewrite the step against it.`,
					blocking: false,
					settled: true
				}
			: undefined
	}
	const match = catalog.options.find((option) => option.resourceRef === resource)
	if (!match) {
		return {
			message: `provider.resource "${resource}" is not one of this workspace's AI provider resources`,
			blocking: catalog.resourcesAreComplete
		}
	}
	if (match.kind !== kind) {
		return blocking(
			`provider.kind "${kind}" does not match "${resource}", which is a \`${match.kind}\` resource`
		)
	}
	if (match.modelsAreLive && !match.models.ids.includes(model)) {
		return {
			message: `model "${model}" is not in the model listing of "${resource}"`,
			blocking: rulesOutOtherModels(match)
		}
	}
	return undefined
}

/**
 * Reject AI agent steps whose provider config would fail at run time: a malformed provider, a
 * resource that is not an AI provider resource of the workspace, or a model the endpoint's own
 * listing rules out.
 *
 * Everything the catalog could not establish is reported through `warnings` instead of blocking,
 * so an incomplete catalog never rejects a provider that would have worked.
 */
export function validateAiAgentProviders(
	modules: unknown,
	catalog: AiAgentProviderCatalog | undefined,
	warnings?: string[]
): void {
	const known = catalog ?? { options: [], resourcesAreComplete: false }
	const errors: string[] = []
	forEachAiAgentModule(modules, (mod, value) => {
		if (value.agent) return
		const transform = value.input_transforms?.provider
		// A missing provider is reported by collectProviderlessAgentIds, and a javascript
		// transform resolves at run time with no value to check here.
		if (!transform || transform.type !== 'static') return
		const issue = checkProviderValue(transform.value, known)
		if (!issue) return
		if (issue.blocking) {
			errors.push(`Step "${mod.id}": ${issue.message}`)
		} else if (issue.settled) {
			warnings?.push(`Step "${mod.id}": ${issue.message}`)
		} else {
			warnings?.push(
				`Step "${mod.id}": ${issue.message}, which could not be ruled out. Confirm it with the user if the step fails to run.`
			)
		}
	})
	if (errors.length > 0) {
		throw new Error(
			`Invalid AI agent provider configuration:\n${errors.join('\n')}${knownOptionsHint(known)}`
		)
	}
}
