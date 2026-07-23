import {
	Gauge,
	Database,
	Timer,
	Hourglass,
	Repeat,
	Cpu,
	Trash2,
	ChevronsUp,
	Tag
} from 'lucide-svelte'
import type { ScriptLang } from '$lib/gen'

// Subset of Script/NewScript fields that make up the "advanced runtime settings"
// surfaced both in the standalone script editor and, via the mini settings drawer,
// from within the flow editor for workspace-script steps.
export type ScriptAdvancedSettingsFields = {
	path?: string
	language?: ScriptLang
	schema?: unknown
	tag?: string
	concurrent_limit?: number
	concurrency_time_window_s?: number
	concurrency_key?: string
	cache_ttl?: number
	cache_ignore_s3_path?: boolean
	timeout?: number
	debounce_delay_s?: number
	debounce_key?: string
	debounce_args_to_accumulate?: string[]
	max_total_debouncing_time?: number
	max_total_debounces_amount?: number
	restart_unless_cancelled?: boolean
	dedicated_worker?: boolean
	delete_after_secs?: number
	priority?: number
}

export type ScriptSettingsBadge = {
	key: string
	label: string
	icon: any
	detail: string
}

// Compute the list of active advanced settings for a script, used to render
// at-a-glance badges in the editor top bar and in the flow drawers.
export function getActiveScriptSettingsBadges(
	settings: ScriptAdvancedSettingsFields | undefined
): ScriptSettingsBadge[] {
	if (!settings) return []
	const badges: ScriptSettingsBadge[] = []
	if (settings.concurrent_limit != undefined) {
		badges.push({
			key: 'concurrency',
			label: 'Concurrency',
			icon: Gauge,
			detail: `Max ${settings.concurrent_limit} execution${
				settings.concurrent_limit === 1 ? '' : 's'
			}${
				settings.concurrency_time_window_s != undefined
					? ` / ${settings.concurrency_time_window_s}s`
					: ''
			}`
		})
	}
	if (settings.cache_ttl != undefined) {
		badges.push({
			key: 'cache',
			label: 'Cache',
			icon: Database,
			detail: `Cached for ${settings.cache_ttl}s`
		})
	}
	if (settings.timeout != undefined) {
		badges.push({
			key: 'timeout',
			label: 'Timeout',
			icon: Timer,
			detail: `${settings.timeout}s`
		})
	}
	if (settings.debounce_delay_s != undefined && settings.debounce_delay_s > 0) {
		badges.push({
			key: 'debounce',
			label: 'Debounce',
			icon: Hourglass,
			detail: `Debounced by ${settings.debounce_delay_s}s`
		})
	}
	if (settings.restart_unless_cancelled) {
		badges.push({
			key: 'perpetual',
			label: 'Perpetual',
			icon: Repeat,
			detail: 'Restarts unless cancelled'
		})
	}
	if (settings.dedicated_worker) {
		badges.push({
			key: 'dedicated',
			label: 'Dedicated',
			icon: Cpu,
			detail: 'Runs on dedicated workers'
		})
	}
	if (settings.delete_after_secs != undefined) {
		badges.push({
			key: 'delete_after_use',
			label: 'Delete after use',
			icon: Trash2,
			detail:
				settings.delete_after_secs === 0
					? 'Deleted immediately after completion'
					: `Deleted ${settings.delete_after_secs}s after completion`
		})
	}
	if (settings.priority != undefined && settings.priority > 0) {
		badges.push({
			key: 'priority',
			label: 'High priority',
			icon: ChevronsUp,
			detail: `Priority ${settings.priority}`
		})
	}
	if (settings.tag) {
		badges.push({
			key: 'tag',
			label: settings.tag,
			icon: Tag,
			detail: `Worker tag: ${settings.tag}`
		})
	}
	return badges
}
