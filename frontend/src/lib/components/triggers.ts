import type { CaptureTriggerKind, TriggersCount } from '$lib/gen'
import type { Writable } from 'svelte/store'

export type ScheduleTrigger = {
	summary: string | undefined
	args: Record<string, any>
	cron: string
	timezone: string
	enabled: boolean
}

export type TriggerContext = {
	selectedTrigger: Writable<TriggerKind>
	primarySchedule: Writable<ScheduleTrigger | undefined | false>
	triggersCount: Writable<TriggersCount | undefined>
	simplifiedPoll: Writable<boolean | undefined>
	defaultValues: Writable<Record<string, any> | undefined>
	captureOn: Writable<boolean | undefined>
	showCaptureHint: Writable<boolean | undefined>
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

export type TriggerKind =
	| 'webhooks'
	| 'emails'
	| 'schedules'
	| 'cli'
	| 'routes'
	| 'websockets'
	| 'scheduledPoll'
	| 'kafka'
	| 'nats'
	| 'postgres'
	| 'mqtt'
	| 'sqs'
export function captureTriggerKindToTriggerKind(kind: CaptureTriggerKind): TriggerKind {
	switch (kind) {
		case 'webhook':
			return 'webhooks'
		case 'email':
			return 'emails'
		case 'http':
			return 'routes'
		case 'websocket':
			return 'websockets'
		case 'kafka':
			return 'kafka'
		case 'nats':
			return 'nats'
		case 'mqtt':
			return 'mqtt'
		case 'sqs':
			return 'sqs'
		case 'postgres':
			return  'postgres'
		default:
			throw new Error(`Unknown CaptureTriggerKind: ${kind}`)
	}
}
