import type { AIProvider } from '$lib/gen'
import { AI_PROVIDERS } from './lib'

/** Whether a resource of this type and value reaches Claude or Gemini through Vertex AI with an
 *  access token it stores, which `vertex_ai_federated` would let it stop storing. */
export function isTokenVertexResource(
	resourceType: string | undefined,
	value: Record<string, any> | undefined
): boolean {
	return (
		(resourceType === 'anthropic' || resourceType === 'googleai') &&
		value?.platform === 'google_vertex_ai'
	)
}

/** A resource type that authenticates the same provider another way, offered as a switch when
 *  creating a resource of the first, since a picker scoped to a provider only creates its main
 *  type. `carry` names the fields the switch keeps. */
export function aiResourceTypeAlternative(
	resourceType: string,
	value: Record<string, any> | undefined
): { resourceType: string; label: string; carry?: string[] } | undefined {
	if (isTokenVertexResource(resourceType, value)) {
		return {
			resourceType: 'vertex_ai_federated',
			label: 'Use Workload Identity Federation instead of an access token',
			carry: ['base_url']
		}
	}
	if (resourceType === 'anthropic') {
		return {
			resourceType: 'anthropic_federated',
			label: 'Authenticate with Workload Identity Federation instead of an API key'
		}
	}
	if (resourceType === 'anthropic_federated') {
		return { resourceType: 'anthropic', label: 'Use an API key instead' }
	}
	return undefined
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
