import type { FlowModule } from '$lib/gen'

/**
 * Detects which advanced settings are active on a flow module
 * Used to show badge indicators on settings tabs
 */
export function getActiveAdvancedSettings(flowModule: FlowModule) {
	const hasConcurrencySetting =
		flowModule.value.type === 'rawscript' &&
		(flowModule.value.concurrent_limit !== undefined ||
			flowModule.value.concurrency_time_window_s !== undefined ||
			flowModule.value.custom_concurrency_key !== undefined)

	const hasTimeoutSetting = Boolean(flowModule.timeout)
	const hasPrioritySetting = flowModule.priority !== undefined && flowModule.priority > 0
	const hasDeleteAfterUseSetting = Boolean(flowModule.delete_after_use)

	return {
		retry: flowModule.retry !== undefined,
		cache: Boolean(flowModule.cache_ttl),
		earlyStop: Boolean(flowModule.stop_after_if || flowModule.stop_after_all_iters_if),
		skip: Boolean(flowModule.skip_if),
		suspend: Boolean(flowModule.suspend),
		sleep: Boolean(flowModule.sleep),
		mock: Boolean(flowModule.mock?.enabled),
		timeout: hasTimeoutSetting,
		priority: hasPrioritySetting,
		deleteAfterUse: hasDeleteAfterUseSetting,
		concurrency: hasConcurrencySetting,
		runtime: hasTimeoutSetting || hasPrioritySetting || hasDeleteAfterUseSetting || hasConcurrencySetting
	}
}

/**
 * Detects if any advanced setting is active on a flow module
 */
export function hasAnyAdvancedSetting(flowModule: FlowModule): boolean {
	const settings = getActiveAdvancedSettings(flowModule)
	return Object.values(settings).some((value) => value === true)
}
