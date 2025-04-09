import { Webhook, Mail, Calendar, Route, Unplug, Database } from 'lucide-svelte'
import KafkaIcon from '$lib/components/icons/KafkaIcon.svelte'
import NatsIcon from '$lib/components/icons/NatsIcon.svelte'
import MqttIcon from '$lib/components/icons/MqttIcon.svelte'
import AwsIcon from '$lib/components/icons/AwsIcon.svelte'
import type { CaptureTriggerKind } from '$lib/gen/types.gen'

export type TriggerType =
	| 'webhook'
	| 'email'
	| 'schedule'
	| 'http'
	| 'websocket'
	| 'postgres'
	| 'kafka'
	| 'nats'
	| 'mqtt'
	| 'sqs'
	| 'primary_schedule'

export type Trigger = { path: string; type: TriggerType; isDraft?: boolean; isPrimary?: boolean }

// Map of trigger kinds to icons
export const triggerIconMap = {
	webhook: Webhook,
	email: Mail,
	schedule: Calendar,
	http: Route,
	websocket: Unplug,
	postgres: Database,
	kafka: KafkaIcon,
	nats: NatsIcon,
	mqtt: MqttIcon,
	sqs: AwsIcon,
	primary_schedule: Calendar
}

/**
 * Converts a TriggerType to a CaptureTriggerKind when a mapping exists
 * @param triggerType The trigger type to convert
 * @returns The corresponding CaptureTriggerKind or undefined if no mapping exists
 */
export function triggerTypeToCaptureKind(triggerType: TriggerType): CaptureTriggerKind | undefined {
	// The types that don't map to CaptureTriggerKind
	const nonCaptureTriggerTypes = ['schedule', 'primary_schedule']

	if (nonCaptureTriggerTypes.includes(triggerType)) {
		return undefined
	}

	// Since we've filtered out non-capturable types, we can safely assert this as CaptureTriggerKind
	return triggerType as CaptureTriggerKind
}
