import { ScheduleService, type Schedule, type TriggersCount } from '$lib/gen'
import type { ScheduleTrigger } from '../triggers'
import type { Writable } from 'svelte/store'
import { get } from 'svelte/store'
import { sendUserToast } from '$lib/utils'

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
		summary: schedule.summary,
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
	loadPrimarySchedule: boolean = false
) {
	if (!path || path == '') {
		schedules.set([])
		primarySchedule.update((ps) => (ps === undefined ? false : ps))
		initialPrimarySchedule.set(structuredClone(get(primarySchedule)))
		return
	}
	try {
		const allSchedules = await ScheduleService.listSchedules({
			workspace,
			path: path,
			isFlow
		})
		const primary = allSchedules.find((s) => s.path == path)
		let remotePrimarySchedule: ScheduleTrigger | false | undefined = undefined
		if (loadPrimarySchedule && primary) {
			remotePrimarySchedule = await loadSchedule(path, workspace)
		} else {
			remotePrimarySchedule = primary
				? {
						summary: primary.summary,
						args: primary.args ?? {},
						cron: primary.schedule,
						timezone: primary.timezone,
						enabled: primary.enabled
				  }
				: false
		}
		primarySchedule.update((ps) => (ps === undefined || forceRefresh ? remotePrimarySchedule : ps))
		initialPrimarySchedule.set(structuredClone(remotePrimarySchedule))

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
