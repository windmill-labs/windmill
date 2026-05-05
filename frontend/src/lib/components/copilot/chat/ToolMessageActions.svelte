<script lang="ts">
	import { Button } from '$lib/components/common'
	import { Calendar, Database, Route, SquarePen, Unplug, Webhook } from 'lucide-svelte'
	import AwsIcon from '$lib/components/icons/AwsIcon.svelte'
	import AzureIcon from '$lib/components/icons/AzureIcon.svelte'
	import GoogleCloudIcon from '$lib/components/icons/GoogleCloudIcon.svelte'
	import KafkaIcon from '$lib/components/icons/KafkaIcon.svelte'
	import MqttIcon from '$lib/components/icons/MqttIcon.svelte'
	import NatsIcon from '$lib/components/icons/NatsIcon.svelte'
	import { runToolDisplayAction } from './createdResourceActions.svelte'
	import type { CreatedResourceTriggerKind, ToolDisplayAction } from './shared'

	interface Props {
		actions: ToolDisplayAction[]
	}

	type ActionCardKey = 'schedule' | CreatedResourceTriggerKind
	type ActionCardConfig = {
		title: string
		icon: any
	}

	let { actions }: Props = $props()
	let runningActionId: string | undefined = $state(undefined)

	const actionCardConfigs: Record<ActionCardKey, ActionCardConfig> = {
		schedule: { title: 'Schedule', icon: Calendar },
		http: { title: 'HTTP trigger', icon: Route },
		websocket: { title: 'WebSocket trigger', icon: Unplug },
		postgres: { title: 'Postgres trigger', icon: Database },
		kafka: { title: 'Kafka trigger', icon: KafkaIcon },
		nats: { title: 'NATS trigger', icon: NatsIcon },
		mqtt: { title: 'MQTT trigger', icon: MqttIcon },
		sqs: { title: 'SQS trigger', icon: AwsIcon },
		gcp: { title: 'GCP Pub/Sub trigger', icon: GoogleCloudIcon },
		azure: { title: 'Azure Event Grid trigger', icon: AzureIcon }
	}

	function getActionCardConfig(action: ToolDisplayAction): ActionCardConfig {
		const key = action.resource === 'schedule' ? 'schedule' : action.triggerKind
		return key
			? actionCardConfigs[key]
			: { title: action.label.replace(/^Open\s+/i, ''), icon: Webhook }
	}

	async function handleAction(action: ToolDisplayAction) {
		if (runningActionId) {
			return
		}
		runningActionId = action.id
		try {
			await runToolDisplayAction(action)
		} finally {
			runningActionId = undefined
		}
	}
</script>

{#if actions.length > 0}
	<div class="space-y-2">
		{#each actions as action (action.id)}
			{@const card = getActionCardConfig(action)}
			{@const Icon = card.icon}
			<div class="flex items-center gap-3 rounded-md border !border-green-500/40 bg-surface p-3">
				<div
					class="flex h-10 w-10 shrink-0 items-center justify-center rounded-md bg-blue-50 text-blue-600 dark:bg-blue-500/10 dark:text-blue-300"
				>
					<Icon class="h-5 w-5" />
				</div>
				<div class="min-w-0 flex-1">
					<div class="truncate text-xs font-semibold text-primary">{card.title}</div>
					<div class="truncate text-2xs text-secondary">{action.path}</div>
				</div>
				<Button
					size="xs"
					variant="default"
					title={action.label}
					loading={runningActionId === action.id}
					disabled={runningActionId !== undefined && runningActionId !== action.id}
					startIcon={{ icon: SquarePen }}
					onClick={() => handleAction(action)}
				>
					Open
				</Button>
			</div>
		{/each}
	</div>
{/if}
