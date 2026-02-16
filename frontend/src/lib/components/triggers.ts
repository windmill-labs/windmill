import type { CaptureTriggerKind, TriggersCount } from '$lib/gen'
import { type Writable } from 'svelte/store'
import { formatCron } from '$lib/utils'
import { Triggers } from './triggers/triggers.svelte'

export type ScheduleTrigger = {
	summary: string | undefined
	description?: string
	args: Record<string, any>
	cron: string
	timezone: string
	enabled: boolean
}

export type TriggerContext = {
	triggersCount: Writable<TriggersCount | undefined>
	simplifiedPoll: Writable<boolean | undefined>
	showCaptureHint: Writable<boolean | undefined>
	triggersState: Triggers
}

export function setScheduledPollSchedule(
	triggersState: Triggers,
	triggersCount: Writable<TriggersCount | undefined>
) {
	const primarySchedule = triggersState.triggers.findIndex((t) => t.isPrimary)
	if (primarySchedule !== -1) {
		triggersState.selectedTriggerIndex = primarySchedule
	} else {
		const draftCfg = {
			enabled: true,
			summary: 'Check for new events every 5 minutes',
			schedule: formatCron('0 */5 * * * *'),
			timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
			args: {},
			is_flow: true
		}

		triggersState.addDraftTrigger(triggersCount, 'schedule', undefined, draftCfg)
		triggersState.selectedTriggerIndex = triggersState.triggers.length - 1
	}
}

export type TriggerKind =
	| 'webhooks'
	| 'emails'
	| 'default_emails'
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
	| 'gcp'
	| 'nextcloud'
	| 'google'
export function captureTriggerKindToTriggerKind(kind: CaptureTriggerKind): TriggerKind {
	switch (kind) {
		case 'webhook':
			return 'webhooks'
		case 'email':
			return 'emails'
		case 'default_email':
			return 'default_emails'
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
			return 'postgres'
		case 'gcp':
			return 'gcp'
		default:
			throw new Error(`Unknown CaptureTriggerKind: ${kind}`)
	}
}
