import type { Input, InputTopRun, RunStartedBy, RunnableType } from '$lib/gen'
import { Bot, Code, LayoutDashboard, ShieldAlert } from 'lucide-svelte'
import BarsStaggered from './icons/BarsStaggered.svelte'
import { triggerDisplayNamesMap, triggerIconMapMono } from './triggers/utils'
import { logFeatureUsage } from '$lib/utils/featureUsage'

export const TOP_LEVEL_CATEGORIES: RunStartedBy[] = [
	'manual',
	'api',
	'schedule',
	'app',
	'trigger',
	'preview'
]
export const INSIDE_CATEGORIES: RunStartedBy[] = [
	'flow_step',
	'agent_tool',
	'script_call',
	'handler'
]

export const CATEGORY_LABELS: Record<RunStartedBy, { label: string; hint?: string }> = {
	manual: { label: 'Manual runs', hint: 'Run from the UI' },
	api: { label: 'API and webhooks', hint: 'CLI, tokens, MCP' },
	schedule: { label: 'Schedules' },
	app: { label: 'Apps' },
	trigger: { label: 'Triggers', hint: 'HTTP, Kafka, email…' },
	preview: { label: 'Editor previews' },
	flow_step: { label: 'Flow steps', hint: 'Loops, branches and subflows included' },
	agent_tool: { label: 'AI agent tools' },
	script_call: { label: 'Script calls', hint: 'SDK, workflow-as-code tasks' },
	handler: { label: 'Error and success handlers' }
}

/** Flows cannot be AI agent tools: the agent rejects them. */
export function availableCategories(runnableType: RunnableType | undefined): RunStartedBy[] {
	const all = [...TOP_LEVEL_CATEGORIES, ...INSIDE_CATEGORIES]
	return runnableType === 'FlowPath' ? all.filter((c) => c !== 'agent_tool') : all
}

/** Selected categories, stored as a comma-separated list; unknown values are dropped. */
export function parseCategories(stored: string): RunStartedBy[] {
	const known = new Set<string>(availableCategories(undefined))
	return stored.split(',').filter((c): c is RunStartedBy => known.has(c))
}

export function isDefaultSelection(selected: RunStartedBy[]): boolean {
	return (
		selected.length === TOP_LEVEL_CATEGORIES.length &&
		TOP_LEVEL_CATEGORIES.every((c) => selected.includes(c))
	)
}

/** What the list holds, for the filter button. */
export function selectionLabel(selected: RunStartedBy[], available: RunStartedBy[]): string {
	const shown = available.filter((c) => selected.includes(c))
	if (isDefaultSelection(shown)) return 'Top-level runs'
	if (shown.length === available.length) return 'Everything'
	if (shown.length === 0) return 'No runs'
	if (shown.length === 1) return CATEGORY_LABELS[shown[0]].label
	return `${shown.length} kinds`
}

const HANDLER_LABELS: Record<string, string> = {
	error_handler: 'Error handler',
	schedule_error_handler: 'Schedule error handler',
	schedule_recovery_handler: 'Recovery handler',
	success_handler: 'Success handler',
	global: 'Instance error handler'
}

/** Icon and tooltip for how a run started. Manual runs and previews have none: the first is
 * the common case and the second keeps its "P" badge. */
export function startedByIcon(run: Input): { icon: any; title: string } | undefined {
	switch (run.started_by) {
		case 'api':
			return { icon: triggerIconMapMono.webhook, title: 'Started through the API' }
		case 'schedule':
			return {
				icon: triggerIconMapMono.schedule,
				title: run.trigger ? `Schedule ${run.trigger}` : 'Schedule'
			}
		case 'app':
			return { icon: LayoutDashboard, title: run.trigger ? `App ${run.trigger}` : 'App' }
		case 'trigger': {
			const kind = run.trigger_kind ?? ''
			const name = (triggerDisplayNamesMap as Record<string, string>)[kind] ?? 'Trigger'
			const icon = (triggerIconMapMono as Record<string, any>)[kind]
			return icon ? { icon, title: run.trigger ? `${name} ${run.trigger}` : name } : undefined
		}
		case 'flow_step':
			return { icon: BarsStaggered, title: 'Flow step' }
		case 'agent_tool':
			return { icon: Bot, title: 'AI agent tool' }
		case 'script_call':
			return { icon: Code, title: 'Script call' }
		case 'handler':
			return { icon: ShieldAlert, title: HANDLER_LABELS[run.created_by] ?? 'Handler' }
	}
	return undefined
}

/** Icon and tooltip for what started a group's top-level run. */
export function topRunIcon(top: InputTopRun): { icon: any; title: string } | undefined {
	const kind = top.trigger_kind
	if (!kind || kind === 'ui') return undefined
	if (kind === 'webhook')
		return { icon: triggerIconMapMono.webhook, title: 'Started through the API' }
	const name = (triggerDisplayNamesMap as Record<string, string>)[kind] ?? kind
	const icon = (triggerIconMapMono as Record<string, any>)[kind]
	return icon ? { icon, title: top.trigger ? `${name} ${top.trigger}` : name } : undefined
}

/** What an inner row shows where a top-level run shows its author. */
export function innerRunLabel(run: Input): string {
	if (run.retry != undefined) return `Retry ${run.retry}`
	switch (run.started_by) {
		case 'flow_step':
			return run.flow_step_id ? `Step ${run.flow_step_id}` : 'Flow step'
		case 'agent_tool':
			return 'Tool call'
		case 'script_call':
			return run.parent_path ? `Called by ${shortPath(run.parent_path)}` : 'Script call'
		case 'handler':
			return HANDLER_LABELS[run.created_by] ?? 'Handler'
	}
	return run.created_by
}

/** A path cut from the left, keeping its last segment: u/admin/etl/load becomes …/load. */
export function shortPath(path: string | undefined): string {
	if (!path) return ''
	const i = path.lastIndexOf('/')
	return i > 0 ? `…/${path.slice(i + 1)}` : path
}

export function lastSegment(path: string): string {
	return path.slice(path.lastIndexOf('/') + 1)
}

export function shortId(id: string): string {
	return id.slice(-6)
}

/** Text for the runs an entry holds besides itself: "+2 retries", "+1 handler", "+3 runs". */
export function innerRunsLabel(entry: Input): string | undefined {
	const group = entry.group
	if (!group || group.runs === 0) return undefined
	const plus = group.complete ? '' : '+'
	const noun =
		group.retries === group.runs
			? group.runs === 1
				? 'retry'
				: 'retries'
			: group.handlers === group.runs
				? group.runs === 1
					? 'handler'
					: 'handlers'
				: group.runs === 1
					? 'run'
					: 'runs'
	return `+${group.runs}${plus} ${noun}`
}

/** A history entry standing for several runs inside another top-level run, opened rather
 * than selected. */
export function isGroupEntry(entry: Input): boolean {
	return !!entry.group && !!entry.top_run
}

// Anonymous counters for the History panel. Same rules as every other `logFeatureUsage`
// caller: the keys below are the whole vocabulary, and no path or run id ever reaches here.

/** A category ticked or unticked in the filter, or the filter reset to its default. */
export type RunHistoryFilterEvent = `${RunStartedBy}:${'on' | 'off'}` | 'reset'

export type RunHistoryGroupEvent =
	/** A group row: this runnable's runs inside another top-level run. */
	| 'group'
	/** A run's "+N" button: its retries, handlers or the runs inside it. */
	| 'run'

export function logRunHistoryFilter(event: RunHistoryFilterEvent): void {
	logFeatureUsage('run_history', 'filter', { key: event })
}

export function logRunHistoryGroupExpand(event: RunHistoryGroupEvent): void {
	logFeatureUsage('run_history', 'group_expand', { key: event })
}
