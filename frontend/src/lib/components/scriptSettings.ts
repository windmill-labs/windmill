import {
	BellOff,
	ChevronsUp,
	CircleStop,
	Combine,
	CornerDownLeft,
	Cpu,
	Database,
	EyeOff,
	FolderOpen,
	Gauge,
	KeyRound,
	Repeat,
	Tag,
	Timer,
	Trash2,
	UserCog
} from 'lucide-svelte'
import type { FlowValue, ScriptLang } from '$lib/gen'

// Subset of Script/NewScript fields that make up the runtime settings surfaced both in the
// standalone script editor and, via the mini settings drawer, from within the flow editor for
// workspace-script steps.
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
	ws_error_handler_muted?: boolean
	envs?: string[]
}

export type ScriptSettingsBadge = {
	key: string
	label: string
	icon: any
	detail: string
}

/** Runtime settings only a flow has. */
export type FlowOnlyRuntimeFields = {
	skip_expr?: string
	early_return?: string
	same_worker?: boolean
}

/** A flow's runtime settings, which it splits between itself and its value, in the shape the
 *  badges read. */
export function flowRuntimeFields(flow: {
	tag?: string
	dedicated_worker?: boolean
	ws_error_handler_muted?: boolean
	value: FlowValue
}): ScriptAdvancedSettingsFields & FlowOnlyRuntimeFields {
	const v = flow.value
	return {
		tag: flow.tag,
		dedicated_worker: flow.dedicated_worker,
		ws_error_handler_muted: flow.ws_error_handler_muted,
		concurrent_limit: v.concurrent_limit,
		concurrency_time_window_s: v.concurrency_time_window_s,
		cache_ttl: v.cache_ttl,
		debounce_delay_s: v.debounce_delay_s,
		delete_after_secs: v.delete_after_secs,
		priority: v.priority,
		skip_expr: v.skip_expr,
		early_return: v.early_return,
		same_worker: v.same_worker
	}
}

// The active runtime settings of a script or flow, in the order their Runtime tab lists them,
// for the at-a-glance badges of the editor top bar, the settings tabs and the flow drawers.
export function getActiveScriptSettingsBadges(
	settings: (ScriptAdvancedSettingsFields & FlowOnlyRuntimeFields) | undefined
): ScriptSettingsBadge[] {
	if (!settings) return []
	const badges: ScriptSettingsBadge[] = []
	if (settings.skip_expr) {
		badges.push({
			key: 'early_stop',
			label: 'Stop flow if',
			icon: CircleStop,
			detail: 'Stops early when the inputs match the expression'
		})
	}
	if (settings.early_return) {
		badges.push({
			key: 'early_return',
			label: 'Early return',
			icon: CornerDownLeft,
			detail: `Sync webhooks return the result of step ${settings.early_return}`
		})
	}
	// Non-positive concurrent_limit / timeout are treated as unset by the runtime
	// (legacy zero rows), so don't surface them as active settings.
	if (settings.timeout != undefined && settings.timeout > 0) {
		badges.push({
			key: 'timeout',
			label: 'Timeout',
			icon: Timer,
			detail: `${settings.timeout}s`
		})
	}
	if (settings.concurrent_limit != undefined && settings.concurrent_limit > 0) {
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
	if (settings.debounce_delay_s != undefined && settings.debounce_delay_s > 0) {
		badges.push({
			key: 'debounce',
			label: 'Debounce',
			icon: Combine,
			detail: `Debounced by ${settings.debounce_delay_s}s`
		})
	}
	if (settings.cache_ttl != undefined && settings.cache_ttl > 0) {
		badges.push({
			key: 'cache',
			label: 'Cache',
			icon: Database,
			detail: `Cached for ${settings.cache_ttl}s`
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
	if (settings.restart_unless_cancelled) {
		badges.push({
			key: 'perpetual',
			label: 'Perpetual',
			icon: Repeat,
			detail: 'Restarts unless cancelled'
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
	if (settings.ws_error_handler_muted) {
		badges.push({
			key: 'error_handler_muted',
			label: 'Error handler muted',
			icon: BellOff,
			detail: 'Failures do not trigger the workspace error handler'
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
	if (settings.dedicated_worker) {
		badges.push({
			key: 'dedicated',
			label: 'Dedicated',
			icon: Cpu,
			detail: 'Runs on dedicated workers'
		})
	}
	if (settings.same_worker) {
		badges.push({
			key: 'same_worker',
			label: 'Shared directory',
			icon: FolderOpen,
			detail: 'Steps run on the same worker and share ./shared'
		})
	}
	return badges
}

// The active settings of a script's or flow's Permissions tab, in the order the tab lists them.
export function getActivePermissionBadges(settings: {
	on_behalf_of_email?: string
	job_token_scopes?: string[] | null
	visible_to_runner_only?: boolean
}): ScriptSettingsBadge[] {
	const badges: ScriptSettingsBadge[] = []
	if (settings.on_behalf_of_email) {
		badges.push({
			key: 'on_behalf_of',
			label: 'Run on behalf of',
			icon: UserCog,
			detail: settings.on_behalf_of_email
		})
	}
	if (settings.job_token_scopes != null) {
		badges.push({
			key: 'job_token',
			label: 'Restricted job token',
			icon: KeyRound,
			detail:
				settings.job_token_scopes.length === 0
					? 'No API access'
					: settings.job_token_scopes.join(', ')
		})
	}
	if (settings.visible_to_runner_only) {
		badges.push({
			key: 'invisible_runs',
			label: 'Invisible runs',
			icon: EyeOff,
			detail: 'Manual runs are visible only to whoever ran them'
		})
	}
	return badges
}
