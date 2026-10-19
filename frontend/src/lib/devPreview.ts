import type { AIConfig } from './gen'

/**
 * Dev only: how this browser pretends the workspace's AI is set up, so the states a fresh
 * cloud workspace goes through can be seen without reconfiguring one. Set from the
 * `/kitchen_sink/home_preview` page; read nowhere in a production build.
 */
export type DevAiState = 'real' | 'free-tier' | 'free-tier-exhausted' | 'no-key' | 'hidden'

export type DevPreview = { ai: DevAiState; freeTierUsedPct: number }

const KEY = 'devPreview'

export function readDevPreview(): DevPreview | undefined {
	if (!import.meta.env.DEV) return undefined
	try {
		const raw = globalThis.window?.localStorage?.getItem(KEY)
		return raw ? (JSON.parse(raw) as DevPreview) : undefined
	} catch {
		return undefined
	}
}

export function writeDevPreview(preview: DevPreview | undefined): void {
	try {
		if (preview && preview.ai !== 'real') localStorage.setItem(KEY, JSON.stringify(preview))
		else localStorage.removeItem(KEY)
	} catch {}
}

/** The workspace's AI config as the preview pretends it is; unchanged outside dev. */
export function previewAiConfig(config: AIConfig): AIConfig {
	const preview = readDevPreview()
	switch (preview?.ai) {
		case 'free-tier':
			return {
				...config,
				free_tier: { exhausted: false, used_ratio: preview.freeTierUsedPct / 100 }
			}
		case 'free-tier-exhausted':
			return { ...config, providers: {}, free_tier: { exhausted: true, used_ratio: 1 } }
		case 'no-key':
			return { ...config, providers: {}, free_tier: undefined }
		case 'hidden':
			return { ...config, copilot_disabled: true }
		default:
			return config
	}
}
