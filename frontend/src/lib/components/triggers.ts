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
	selectedTrigger: Writable<
		| 'webhooks'
		| 'emails'
		| 'schedules'
		| 'cli'
		| 'routes'
		| 'websockets'
		| 'scheduledPoll'
		| 'kafka'
	>
	primarySchedule: Writable<ScheduleTrigger | undefined | false>
	triggersCount: Writable<TriggersCount | undefined>
	simplifiedPoll: Writable<boolean | undefined>
}

export function setScheduledPollSchedule(
	primarySchedule: Writable<ScheduleTrigger | undefined | false>,
	triggersCount: Writable<TriggersCount | undefined>
) {
	const cron = '0 */5 * * * *'
	primarySchedule.set({
		enabled: true,
		summary: 'Check for new events every 5 minutes',
		cron: cron,
		timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
		args: {}
	})
	triggersCount.update((triggersCount) => {
		return {
			...(triggersCount ?? {}),
			schedule_count: (triggersCount?.schedule_count ?? 0) + 1,
			primary_schedule: { schedule: cron }
		}
	})
}
