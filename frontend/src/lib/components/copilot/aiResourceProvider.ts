import type { AIProvider } from '$lib/gen'
import { AI_PROVIDERS } from './lib'

/** The AI provider a resource of this type holds credentials for, if any. */
export function aiResourceProvider(resourceType: string | undefined): AIProvider | undefined {
	if (resourceType === 'openai_client_credentials_oauth') return 'openai'
	return resourceType && resourceType in AI_PROVIDERS ? (resourceType as AIProvider) : undefined
}
