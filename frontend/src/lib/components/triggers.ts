import type { CaptureTriggerKind, TriggersCount } from '$lib/gen'
import { get, type Writable } from 'svelte/store'
import { addDraftTrigger, type Trigger } from './triggers/utils'
import { formatCron } from '$lib/utils'

export type ScheduleTrigger = {
	summary: string | undefined
	description?: string
	args: Record<string, any>
	cron: string
	timezone: string
	enabled: boolean
}

export type TriggerContext = {
	selectedTrigger: Writable<Trigger | undefined>
	triggersCount: Writable<TriggersCount | undefined>
	simplifiedPoll: Writable<boolean | undefined>

	captureOn: Writable<boolean | undefined>
	showCaptureHint: Writable<boolean | undefined>
	triggers: Writable<Trigger[]>
}

export function setScheduledPollSchedule(
	triggers: Writable<Trigger[]>,
	triggersCount: Writable<TriggersCount | undefined>
) {
	const primarySchedule = get(triggers).find((t) => t.isPrimary)
	if (primarySchedule) {
		return
	} else {
		const draftCfg = {
			enabled: true,
			summary: 'Check for new events every 5 minutes',
			schedule: formatCron('0 */5 * * * *'),
			timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
			args: {},
			is_flow: true
		}

		addDraftTrigger(triggers, triggersCount, 'schedule', undefined, draftCfg)
	}
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
	| 'gcp'
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
			return 'postgres'
		case 'gcp':
			return 'gcp'
		default:
			throw new Error(`Unknown CaptureTriggerKind: ${kind}`)
	}
}
