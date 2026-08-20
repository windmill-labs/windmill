import type { AIProvider } from '$lib/gen'
import { forEachAiAgentModule } from '$lib/components/flows/aiAgentModules'

/** One AI provider resource of a workspace, with the models it actually serves.
 * Built by `getAiAgentProviderCatalog`; kept in a module of its own so validation
 * stays pure. */
export type AiAgentProviderOption = {
	kind: AIProvider
	resourcePath: string
	/** `$res:<path>`, the form an `aiagent` module's provider config references. */
	resourceRef: string
	models: string[]
	/** `models` came from the provider's own model listing, so it reflects that endpoint's
	 * names. False when the listing failed and the models are a curated fallback. */
	modelsAreLive: boolean
	/** The resource points at a gateway or proxy rather than the provider's own API (custom
	 * `base_url`, a non-standard platform, or the `customai` kind). Such an endpoint may accept
	 * aliases its listing does not return, so an unlisted model is reported, never rejected. */
	customEndpoint: boolean
	/** Model ids the workspace AI settings selected for this provider, if any. */
	configuredModels: string[]
}

export type AiAgentProviderCatalog = {
	options: AiAgentProviderOption[]
	/** Workspace default model, when its provider is one of `options`. */
	defaultModel?: { kind: AIProvider; model: string }
}

/** Models listed per resource in the prompt. Providers expose hundreds of ids
 * (OpenRouter, Bedrock); the whole list would crowd out the flow instructions. */
const MAX_PROMPTED_MODELS = 25

const PROVIDER_SHAPE =
	'{ "type": "static", "value": { "kind": "<provider kind>", "resource": "$res:<resource path>", "model": "<model id>" } }'

/** Whether an unlisted model id is provably wrong for this resource. */
function enforcesModelIds(option: AiAgentProviderOption): boolean {
	return option.modelsAreLive && !option.customEndpoint
}

function describeOption(option: AiAgentProviderOption): string {
	const models = option.models.slice(0, MAX_PROMPTED_MODELS)
	const more =
		option.models.length > models.length
			? `, ... (${option.models.length - models.length} more)`
			: ''
	const modelList =
		models.length > 0 ? models.join(', ') + more : '(none listed — ask the user which model to use)'
	const endpointNote = option.customEndpoint
		? ' — custom endpoint, so it may also accept model names this list does not show'
		: ''
	return `- \`$res:${option.resourcePath}\` (kind \`${option.kind}\`) — models: ${modelList}${endpointNote}`
}

/** Prompt section listing the providers an `aiagent` module may reference and the
 * exact shape of its provider config. Empty string when nothing is configured, so
 * the caller can drop the section entirely. */
export function formatAiAgentProvidersPrompt(catalog: AiAgentProviderCatalog): string {
	if (catalog.options.length === 0) {
		return ''
	}
	// One resource plus a workspace default leaves nothing to decide. Anything else — several
	// resources to choose between, or no default model — is the user's call, not a guess.
	const unambiguous = catalog.options.length === 1 && catalog.defaultModel !== undefined
	const defaultLine = unambiguous
		? `\nUnless the user asks for something else, use kind \`${catalog.defaultModel!.kind}\` with model \`${catalog.defaultModel!.model}\` — the workspace default. Name the model you used in your reply.`
		: `\nWhen the user has not said which provider resource or model the agent should use, ask with \`askUserQuestion\` before writing the step, offering the resources and models above as proposed answers. Do not pick one yourself.`
	return `## AI agent steps (\`aiagent\` modules)

A standalone \`aiagent\` module configures its model through a \`provider\` input transform whose static value is an object:
\`"provider": ${PROVIDER_SHAPE}\`
A bare \`"$res:..."\` string is not a provider config. Use one of the resources below and one of the model ids listed for it — never a model id from memory: an id the endpoint does not serve 404s at run time, and is rejected when the flow is written unless the resource is a custom endpoint.
A module that links to a saved agent (\`"agent": "<path>"\`) takes that agent's provider and needs no \`provider\` transform of its own.

AI provider resources in this workspace:
${catalog.options.map(describeOption).join('\n')}${defaultLine}`
}

/** Tool-result suffix for non-blocking provider findings. Empty when there are none. */
export function formatAiAgentProviderWarnings(warnings: string[]): string {
	if (warnings.length === 0) return ''
	return `\n\nAI agent provider warning(s):\n${warnings.join('\n')}`
}

function knownOptionsHint(options: AiAgentProviderOption[]): string {
	if (options.length === 0) return ''
	return `\nAI provider resources in this workspace:\n${options.map(describeOption).join('\n')}`
}

type ProviderIssue = { message: string; blocking: boolean }

function blocking(message: string): ProviderIssue {
	return { message, blocking: true }
}

function checkProviderValue(
	value: unknown,
	options: AiAgentProviderOption[]
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
	if (options.length === 0) {
		return undefined
	}
	const match = options.find((option) => option.resourceRef === resource)
	if (!match) {
		return blocking(
			`provider.resource "${resource}" is not an AI provider resource of this workspace`
		)
	}
	if (match.kind !== kind) {
		return blocking(
			`provider.kind "${kind}" does not match "${resource}", which is a \`${match.kind}\` resource`
		)
	}
	if (match.modelsAreLive && !match.models.includes(model)) {
		// A gateway can accept aliases its own listing omits, so there the mismatch is
		// only reported; on the provider's own API an unlisted id is wrong.
		return {
			message: `model "${model}" is not in the model listing of "${resource}"`,
			blocking: enforcesModelIds(match)
		}
	}
	return undefined
}

/**
 * Reject `aiagent` modules whose provider config would fail at run time: a
 * malformed provider, a resource that is not an AI provider resource of the
 * workspace, or a model that resource's own listing rules out. Model ids are only
 * checked against a live listing from a provider-owned endpoint — a failed listing
 * or a proxy resource passes, the latter with a note pushed to `warnings`.
 *
 * `options` empty (no catalog, or none could be loaded) limits this to shape checks.
 */
export function validateAiAgentProviders(
	modules: unknown,
	options: AiAgentProviderOption[] | undefined,
	warnings?: string[]
): void {
	const known = options ?? []
	const errors: string[] = []
	forEachAiAgentModule(modules, (mod, value) => {
		if (value.agent) return
		const transform = value.input_transforms?.provider
		// A missing provider is reported by collectProviderlessAgentIds, and a
		// javascript transform resolves at run time with no value to check here.
		if (!transform || transform.type !== 'static') return
		const issue = checkProviderValue(transform.value, known)
		if (!issue) return
		if (issue.blocking) {
			errors.push(`Module "${mod.id}": ${issue.message}`)
		} else {
			warnings?.push(`Module "${mod.id}": ${issue.message}. Check it against that endpoint.`)
		}
	})
	if (errors.length > 0) {
		throw new Error(
			`Invalid AI agent provider configuration:\n${errors.join('\n')}${knownOptionsHint(known)}`
		)
	}
}
