import { z } from 'zod'
import type { FlowValue } from '$lib/gen'

/**
 * Non-structural top-level FlowValue settings exposed in the compact view so
 * they round-trip through patch/write tools instead of being silently dropped.
 * Must stay in sync with the top-level scalar/config fields of `FlowValue`.
 */
export const flowValueSettingsSchema = z
	.object({
		same_worker: z.boolean(),
		preserve_step_tags: z.boolean(),
		concurrent_limit: z.number(),
		concurrency_key: z.string(),
		concurrency_time_window_s: z.number(),
		debounce_delay_s: z.number(),
		debounce_key: z.string(),
		debounce_args_to_accumulate: z.array(z.string()),
		max_total_debouncing_time: z.number(),
		max_total_debounces_amount: z.number(),
		skip_expr: z.string(),
		cache_ttl: z.number(),
		cache_ignore_s3_path: z.boolean(),
		delete_after_secs: z.number(),
		flow_env: z.record(z.string(), z.any()),
		priority: z.number(),
		early_return: z.string(),
		chat_input_enabled: z.boolean()
	})
	.partial()

type FlowValueSettingsKey = keyof z.infer<typeof flowValueSettingsSchema> & keyof FlowValue

export const FLOW_VALUE_SETTINGS_KEYS = Object.keys(
	flowValueSettingsSchema.shape
) as FlowValueSettingsKey[]

export type FlowValueSettings = Pick<FlowValue, FlowValueSettingsKey>

/**
 * Extract the defined non-structural settings from a FlowValue (or an
 * EditableFlowJson, which carries the same keys).
 */
export function pickFlowValueSettings(source: Record<string, unknown>): FlowValueSettings {
	const settings: Record<string, unknown> = {}
	for (const key of FLOW_VALUE_SETTINGS_KEYS) {
		if (source[key] !== undefined) {
			settings[key] = source[key]
		}
	}
	return settings as FlowValueSettings
}
