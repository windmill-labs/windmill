import type { AIProvider } from '$lib/gen'
import { AI_PROVIDERS } from './lib'

/** A resource type that authenticates the same provider another way, offered as a switch when
 *  creating a resource of the first, since a picker scoped to a provider only creates its main type. */
export const AI_RESOURCE_TYPE_ALTERNATIVES: Record<string, { resourceType: string; label: string }> = {
	anthropic: {
		resourceType: 'anthropic_federated',
		label: 'Authenticate with Workload Identity Federation instead of an API key'
	},
	anthropic_federated: { resourceType: 'anthropic', label: 'Use an API key instead' }
}

/** The AI provider a resource of this type holds credentials for, if any. */
export function aiResourceProvider(resourceType: string | undefined): AIProvider | undefined {
	if (resourceType === 'openai_client_credentials_oauth') return 'openai'
	if (resourceType === 'anthropic_federated') return 'anthropic'
	return resourceType && Object.hasOwn(AI_PROVIDERS, resourceType)
		? (resourceType as AIProvider)
		: undefined
}
