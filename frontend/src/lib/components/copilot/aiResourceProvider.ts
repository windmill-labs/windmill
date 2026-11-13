import type { AIProvider } from '$lib/gen'
import { AI_PROVIDERS } from './lib'

/** A resource type that authenticates the same provider another way, offered as a switch when
 *  creating a resource of the first, since a picker scoped to a provider only creates its main type. */
export const AI_RESOURCE_TYPE_ALTERNATIVES: Record<
	string,
	{ resourceType: string; label: string }
> = {
	anthropic: {
		resourceType: 'anthropic_federated',
		label: 'Authenticate with Workload Identity Federation instead of an API key'
	},
	anthropic_federated: { resourceType: 'anthropic', label: 'Use an API key instead' },
	googleai: {
		resourceType: 'vertex_ai_federated',
		label: 'Use Vertex AI with Workload Identity Federation instead of an API key'
	}
}

/** The provider a Vertex AI endpoint serves, read off its `publishers/<publisher>` segment. */
export function vertexPublisherProvider(baseUrl: unknown): AIProvider | undefined {
	if (typeof baseUrl !== 'string') return undefined
	if (baseUrl.includes('/publishers/anthropic/')) return 'anthropic'
	if (baseUrl.includes('/publishers/google/')) return 'googleai'
	return undefined
}

/** The AI provider a resource of this type holds credentials for, if any. A
 *  `vertex_ai_federated` resource serves both Claude and Gemini, so its provider is the publisher
 *  its `base_url` names, Google when it names none yet. */
export function aiResourceProvider(
	resourceType: string | undefined,
	value?: Record<string, any>
): AIProvider | undefined {
	if (resourceType === 'openai_client_credentials_oauth') return 'openai'
	if (resourceType === 'anthropic_federated') return 'anthropic'
	if (resourceType === 'vertex_ai_federated') {
		return vertexPublisherProvider(value?.base_url) ?? 'googleai'
	}
	return resourceType && Object.hasOwn(AI_PROVIDERS, resourceType)
		? (resourceType as AIProvider)
		: undefined
}
