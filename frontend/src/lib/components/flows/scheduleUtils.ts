import { ScheduleService, type Schedule, type TriggersCount } from '$lib/gen'
import type { ScheduleTrigger } from '../triggers'
import type { Writable } from 'svelte/store'
import { writable } from 'svelte/store'
import { get } from 'svelte/store'
import { sendUserToast } from '$lib/utils'
import { stateSnapshot } from '$lib/svelte5Utils.svelte'

// Load the schedule of a flow given its path and the workspace
export async function loadSchedule(path: string, workspace: string): Promise<ScheduleTrigger> {
	const existsSchedule = await ScheduleService.existsSchedule({
		workspace,
		path
	})

	if (!existsSchedule) {
		throw new Error(`Flow at path: ${path} doesn't exist`)
	}

	const schedule = await ScheduleService.getSchedule({
		workspace,
		path
	})

	return {
		summary: schedule.summary ?? undefined,
		enabled: schedule.enabled,
		cron: schedule.schedule,
		timezone: schedule.timezone,
		args: schedule.args ?? {}
	}
}

export async function loadSchedules(
	forceRefresh: boolean,
	path: string,
	isFlow: boolean,
	schedules: Writable<Schedule[] | undefined>,
	primarySchedule: Writable<ScheduleTrigger | false | undefined>,
	initialPrimarySchedule: Writable<ScheduleTrigger | false | undefined>,
	workspace: string,
	triggersCount: Writable<TriggersCount | undefined>,
	loadPrimarySchedule: boolean = false,
	isDeployed: Writable<boolean | undefined> = writable(undefined)
) {
	if (!path || path == '') {
		schedules.set([])
		primarySchedule.update((ps) => (ps === undefined ? false : ps))
		initialPrimarySchedule.set(structuredClone(stateSnapshot(get(primarySchedule))))
		return
	}
	try {
		const allSchedules = await ScheduleService.listSchedules({
			workspace,
			path: path,
			isFlow
		})
		const primary = allSchedules.find((s) => s.path == path)
		if (primary) {
			isDeployed.set(true)
		}
		let remotePrimarySchedule: ScheduleTrigger | false | undefined = undefined
		if (loadPrimarySchedule && primary) {
			remotePrimarySchedule = await loadSchedule(path, workspace)
		} else {
			remotePrimarySchedule = primary
				? {
						summary: primary.summary ?? undefined,
						args: primary.args ?? {},
						cron: primary.schedule,
						timezone: primary.timezone,
						enabled: primary.enabled
					}
				: false
		}
		primarySchedule.update((ps) => (ps === undefined || forceRefresh ? remotePrimarySchedule : ps))
		initialPrimarySchedule.set(structuredClone(stateSnapshot(remotePrimarySchedule)))

		triggersCount.update((tc) => {
			const primary = get(primarySchedule)
			return {
				...(tc ?? {}),
				schedule_count: allSchedules.length,
				primary_schedule: primary ? { schedule: primary.cron } : undefined
			}
		})
		schedules.set(allSchedules.filter((s) => s.path != path))
	} catch (e) {
		console.error('impossible to load schedules', e)
	}
}

export async function saveSchedule(
	path: string,
	newItem: boolean,
	workspace: string,
	primarySchedule: Writable<ScheduleTrigger | false | undefined>,
	isFlow: boolean
) {
	const scheduleExists =
		path != '' &&
		!newItem &&
		(await ScheduleService.existsSchedule({
			workspace,
			path
		}))
	if (scheduleExists) {
		console.log('primary schedule exists')
		const primary = get(primarySchedule)
		if (primary) {
			await ScheduleService.updateSchedule({
				workspace,
				path,
				requestBody: {
					summary: primary.summary,
					args: primary.args,
					schedule: primary.cron,
					timezone: primary.timezone
				}
			})
			sendUserToast(`Primary schedule updated`)
		} else {
			await ScheduleService.deleteSchedule({ workspace, path })
			sendUserToast(`Primary schedule deleted`)
		}
	} else {
		const primary = get(primarySchedule)
		if (primary) {
			await ScheduleService.createSchedule({
				workspace,
				requestBody: {
					path,
					script_path: path,
					is_flow: isFlow,
					summary: primary.summary,
					args: primary.args,
					schedule: primary.cron,
					timezone: primary.timezone,
					enabled: primary.enabled
				}
			})
			sendUserToast(`Primary schedule created`)
		}
	}
}

export async function saveScheduleFromCfg(
	scheduleCfg: Record<string, any>,
	edit: boolean,
	workspace: string
): Promise<boolean> {
	const requestBody = {
		schedule: scheduleCfg.schedule,
		timezone: scheduleCfg.timezone,
		args: scheduleCfg.args,
		on_failure: scheduleCfg.on_failure,
		on_failure_times: scheduleCfg.on_failure_times,
		on_failure_exact: scheduleCfg.on_failure_exact,
		on_failure_extra_args: scheduleCfg.on_failure_extra_args,
		on_recovery: scheduleCfg.on_recovery,
		on_recovery_times: scheduleCfg.on_recovery_times,
		on_recovery_extra_args: scheduleCfg.on_recovery_extra_args,
		on_success: scheduleCfg.on_success,
		on_success_extra_args: scheduleCfg.on_success_extra_args,
		ws_error_handler_muted: scheduleCfg.ws_error_handler_muted,
		retry: !scheduleCfg.is_flow ? scheduleCfg.retry : undefined,
		summary: scheduleCfg.summary,
		description: scheduleCfg.description,
		no_flow_overlap: scheduleCfg.no_flow_overlap,
		tag: scheduleCfg.tag,
		paused_until: scheduleCfg.paused_until,
		cron_version: scheduleCfg.cron_version,
		dynamic_skip: scheduleCfg.dynamic_skip
	}
	try {
		if (edit) {
			await ScheduleService.updateSchedule({
				workspace,
				path: scheduleCfg.path,
				requestBody: requestBody
			})
			sendUserToast(`Schedule ${scheduleCfg.path} updated`)
		} else {
			await ScheduleService.createSchedule({
				workspace,
				requestBody: {
					path: scheduleCfg.path,
					script_path: scheduleCfg.script_path,
					is_flow: scheduleCfg.is_flow,
					...requestBody,
					enabled: true
				}
			})
			sendUserToast(`Schedule ${scheduleCfg.path} created`)
		}
		return true
	} catch (error) {
		sendUserToast(error.body || error.message, true)
		return false
	}
}
