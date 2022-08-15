import { ScheduleService } from '$lib/gen'

export type Schedule = {
	args: Record<string, any>
	cron: string
	enabled: boolean
	previewArgs: Record<string, any>
}

// Load the schedule of a flow given its path and the workspace
export async function loadFlowSchedule(path: string, workspace: string = ''): Promise<Schedule> {
	const existsSchedule = await ScheduleService.existsSchedule({
		workspace,
		path
	})

	if (!existsSchedule) {
		throw new Error(`Flow at path: ${path} doesn't exit`)
	}

	const schedule = await ScheduleService.getSchedule({
		workspace,
		path
	})

	return {
		enabled: schedule.enabled,
		cron: schedule.schedule,
		args: schedule.args ?? {},
		previewArgs: JSON.parse(JSON.stringify(schedule.args))
	}
}
