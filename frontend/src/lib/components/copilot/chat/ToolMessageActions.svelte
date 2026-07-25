<script lang="ts">
	import { Button } from '$lib/components/common'
	import {
		ArrowRight,
		Boxes,
		Calendar,
		Database,
		DollarSign,
		FolderOpen,
		GitCompareArrows,
		KeyRound,
		Mail,
		Package,
		Play,
		Route,
		ScrollText,
		Settings,
		SquarePen,
		Unplug,
		Users,
		Webhook,
		Zap
	} from 'lucide-svelte'
	import AwsIcon from '$lib/components/icons/AwsIcon.svelte'
	import AzureIcon from '$lib/components/icons/AzureIcon.svelte'
	import GoogleCloudIcon from '$lib/components/icons/GoogleCloudIcon.svelte'
	import KafkaIcon from '$lib/components/icons/KafkaIcon.svelte'
	import MqttIcon from '$lib/components/icons/MqttIcon.svelte'
	import AmqpIcon from '$lib/components/icons/AmqpIcon.svelte'
	import NatsIcon from '$lib/components/icons/NatsIcon.svelte'
	import { runToolDisplayAction } from './createdResourceActions.svelte'
	import type { CreatedResourceTriggerKind, ToolDisplayAction } from './shared'

	interface Props {
		actions: ToolDisplayAction[]
	}

	type ActionCardKey = 'schedule' | 'resource' | 'variable' | CreatedResourceTriggerKind
	type ActionCardConfig = {
		title: string
		icon: any
	}
	type ActionCard = ActionCardConfig & { subtitle: string; buttonIcon: any }

	let { actions }: Props = $props()
	let runningActionId: string | undefined = $state(undefined)

	const actionCardConfigs: Record<ActionCardKey, ActionCardConfig> = {
		schedule: { title: 'Schedule', icon: Calendar },
		resource: { title: 'Resource', icon: Package },
		variable: { title: 'Variable', icon: KeyRound },
		http: { title: 'HTTP trigger', icon: Route },
		websocket: { title: 'WebSocket trigger', icon: Unplug },
		postgres: { title: 'Postgres trigger', icon: Database },
		kafka: { title: 'Kafka trigger', icon: KafkaIcon },
		nats: { title: 'NATS trigger', icon: NatsIcon },
		mqtt: { title: 'MQTT trigger', icon: MqttIcon },
		amqp: { title: 'AMQP trigger', icon: AmqpIcon },
		sqs: { title: 'SQS trigger', icon: AwsIcon },
		gcp: { title: 'GCP Pub/Sub trigger', icon: GoogleCloudIcon },
		azure: { title: 'Azure Event Grid trigger', icon: AzureIcon },
		email: { title: 'Email trigger', icon: Mail }
	}

	const navigatePageConfig: Record<string, ActionCardConfig> = {
		runs: { title: 'Runs', icon: Play },
		schedules: { title: 'Schedules', icon: Calendar },
		variables: { title: 'Variables', icon: DollarSign },
		resources: { title: 'Resources', icon: Boxes },
		assets: { title: 'Assets', icon: Database },
		audit_logs: { title: 'Audit logs', icon: ScrollText },
		folders: { title: 'Folders', icon: FolderOpen },
		groups: { title: 'Groups', icon: Users },
		triggers: { title: 'Triggers', icon: Zap },
		workspace_settings: { title: 'Workspace settings', icon: Settings },
		compare: { title: 'Compare & Deploy', icon: GitCompareArrows }
	}

	function getActionCard(action: ToolDisplayAction): ActionCard {
		if (action.type === 'navigate') {
			const config = navigatePageConfig[action.page] ?? { title: action.page, icon: Route }
			return { ...config, subtitle: action.label, buttonIcon: ArrowRight }
		}
		if (action.type === 'open_item_preview') {
			// Preview-item cards render through ToolPreviewCard, not here; keep this branch
			// only so the exhaustive union stays type-safe.
			return { title: action.label, icon: SquarePen, subtitle: action.path, buttonIcon: ArrowRight }
		}
		const key: ActionCardKey | undefined =
			action.resource === 'trigger' ? action.triggerKind : action.resource
		const config = key
			? actionCardConfigs[key]
			: { title: action.label.replace(/^Open\s+/i, ''), icon: Webhook }
		return { ...config, subtitle: action.path, buttonIcon: SquarePen }
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
			{@const card = getActionCard(action)}
			{@const Icon = card.icon}
			<div class="flex items-center gap-3 rounded-md border !border-green-500/40 bg-surface p-3">
				<div
					class="flex h-10 w-10 shrink-0 items-center justify-center rounded-md bg-blue-50 text-blue-600 dark:bg-blue-500/10 dark:text-blue-300"
				>
					<Icon class="h-5 w-5" />
				</div>
				<div class="min-w-0 flex-1">
					<div class="truncate text-xs font-semibold text-primary">{card.title}</div>
					<div class="truncate text-2xs text-secondary">{card.subtitle}</div>
				</div>
				<Button
					size="xs"
					variant="default"
					title={action.label}
					loading={runningActionId === action.id}
					disabled={runningActionId !== undefined && runningActionId !== action.id}
					startIcon={{ icon: card.buttonIcon }}
					onClick={() => handleAction(action)}
				>
					Open
				</Button>
			</div>
		{/each}
	</div>
{/if}
