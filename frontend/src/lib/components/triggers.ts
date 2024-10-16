import type { TriggersCount } from '$lib/gen'
import type { Writable } from 'svelte/store'

export type ScheduleTrigger = {
	summary: string | undefined
	args: Record<string, any>
	cron: string
	timezone: string
	enabled: boolean
}

export type TriggerContext = {
	selectedTrigger: Writable<'webhooks' | 'emails' | 'schedules' | 'cli' | 'routes'>
	primarySchedule: Writable<ScheduleTrigger | undefined | false>
	triggersCount: Writable<TriggersCount | undefined>
}
