import { AI_PROVIDERS } from './copilot/lib'
import type { ProviderConfig } from '$lib/gen'

const STORAGE_KEY = 'windmill_ai_provider_config'

export function loadStoredConfig(): ProviderConfig | undefined {
	if (typeof localStorage === 'undefined') {
		return undefined
	}
	try {
		const stored = localStorage.getItem(STORAGE_KEY)
		if (stored) {
			const parsed = JSON.parse(stored)
			// Validate that the stored provider is still available
			if (parsed.kind && AI_PROVIDERS[parsed.kind]) {
				return parsed
			}
		}
	} catch (e) {
		console.error('Failed to load AI provider config from localStorage:', e)
	}
	return undefined
}

export function saveConfig(config: ProviderConfig | undefined): void {
	if (typeof localStorage === 'undefined') {
		return
	}
	try {
		localStorage.setItem(STORAGE_KEY, JSON.stringify(config))
	} catch (e) {
		console.error('Failed to save AI provider config to localStorage:', e)
	}
}

export function removeConfig(): void {
	if (typeof localStorage === 'undefined') {
		return
	}
	try {
		localStorage.removeItem(STORAGE_KEY)
	} catch (e) {
		console.error('Failed to remove AI provider config from localStorage:', e)
	}
}

export function isSameAsStoredConfig(config: ProviderConfig | undefined): boolean {
	const storedConfig = loadStoredConfig()
	return (
		storedConfig !== undefined &&
		storedConfig.kind === config?.kind &&
		storedConfig.resource === config?.resource &&
		storedConfig.model === config?.model
	)
}
