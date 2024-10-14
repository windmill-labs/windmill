import type { HttpTrigger, Schedule, FlowModule } from '$lib/gen'
import type { Writable } from 'svelte/store'

export type ScheduleTrigger = {
	summary: string | undefined
	args: Record<string, any>
	cron: string
	timezone: string
	enabled: boolean
}

export type TriggerContext = {
	selectedTrigger: Writable<string>
	httpTriggers: Writable<(HttpTrigger & { canWrite: boolean })[] | undefined>
	schedule: Writable<ScheduleTrigger>
	primarySchedule: Writable<Schedule | undefined | boolean>
	schedules: Writable<Schedule[] | undefined>
	triggerModule: Writable<FlowModule | undefined>
}
