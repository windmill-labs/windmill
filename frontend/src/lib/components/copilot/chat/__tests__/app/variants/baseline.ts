import type { VariantConfig } from '../../shared'

/**
 * Baseline variant - uses the production system prompt and all tools.
 * This is the default configuration that matches the actual app chat implementation.
 */
export const BASELINE_VARIANT: VariantConfig = {
	name: 'baseline',
	description: 'Production configuration with default system prompt and all tools',
	systemPrompt: { type: 'default' },
	tools: { type: 'default' }
}
