import { ScheduleService } from '$lib/gen'
import type { ScheduleTrigger } from '../triggers'

// Load the schedule of a flow given its path and the workspace
export async function loadFlowSchedule(path: string, workspace: string): Promise<ScheduleTrigger> {
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
