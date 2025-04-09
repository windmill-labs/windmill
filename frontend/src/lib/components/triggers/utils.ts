import { Webhook, Mail, Calendar, Route, Unplug, Database } from 'lucide-svelte'
import KafkaIcon from '$lib/components/icons/KafkaIcon.svelte'
import NatsIcon from '$lib/components/icons/NatsIcon.svelte'
import MqttIcon from '$lib/components/icons/MqttIcon.svelte'
import AwsIcon from '$lib/components/icons/AwsIcon.svelte'

export type Trigger = { path: string; type: string; isDraft?: boolean; isPrimary?: boolean }

// Map of trigger kinds to icons
export const triggerIconMap = {
	webhooks: Webhook,
	emails: Mail,
	schedules: Calendar,
	routes: Route, // HTTP
	websockets: Unplug,
	postgres: Database,
	kafka: KafkaIcon,
	nats: NatsIcon,
	mqtt: MqttIcon,
	sqs: AwsIcon,
	primary_schedule: Calendar
}
