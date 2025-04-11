<script lang="ts">
	import { Calendar, Mail, Webhook, Unplug, Database, PlugZap } from 'lucide-svelte'
	import TriggerButton from './TriggerButton.svelte'

	import Popover from '$lib/components/Popover.svelte'
	import TriggerCount from './TriggerCount.svelte'
	import { createEventDispatcher, onMount, type ComponentType } from 'svelte'
	import { Route } from 'lucide-svelte'
	import { getContext } from 'svelte'
	import { type TriggerContext } from '$lib/components/triggers'
	import { FlowService, ScriptService } from '$lib/gen'
	import { enterpriseLicense, workspaceStore } from '$lib/stores'
	import { MqttIcon, NatsIcon, KafkaIcon, AwsIcon } from '$lib/components/icons'
	import GoogleCloudIcon from '$lib/components/icons/GoogleCloudIcon.svelte'

	const { selectedTrigger, triggersCount } = getContext<TriggerContext>('TriggerContext')

	export let path: string
	export let newItem: boolean
	export let isFlow: boolean
	export let selected: boolean
	export let showOnlyWithCount: boolean
	export let triggersToDisplay: (
		| 'webhooks'
		| 'schedules'
		| 'routes'
		| 'websockets'
		| 'kafka'
		| 'nats'
		| 'mqtt'
		| 'emails'
		| 'eventStreams'
		| 'postgres'
		| 'sqs'
		| 'gcp'
	)[] = showOnlyWithCount
		? ['webhooks', 'schedules', 'routes', 'websockets', 'kafka', 'nats', 'emails']
		: ['webhooks', 'schedules', 'routes', 'websockets', 'eventStreams', 'emails']
	const dispatch = createEventDispatcher()

	onMount(() => {
		if (!newItem) {
			loadCount()
		}
	})

	async function loadCount() {
		if (isFlow) {
			$triggersCount = await FlowService.getTriggersCountOfFlow({
				workspace: $workspaceStore!,
				path
			})
		} else {
			$triggersCount = await ScriptService.getTriggersCountOfScript({
				workspace: $workspaceStore!,
				path
			})
		}
	}

	const triggerTypeConfig: {
		[key: string]: { icon: ComponentType; countKey?: string }
	} = {
		webhooks: { icon: Webhook, countKey: 'webhook_count' },
		schedules: { icon: Calendar, countKey: 'schedule_count' },
		routes: { icon: Route, countKey: 'http_routes_count' },
		websockets: { icon: Unplug, countKey: 'websocket_count' },
		postgres: { icon: Database, countKey: 'postgres_count' },
		kafka: { icon: KafkaIcon, countKey: 'kafka_count' },
		emails: { icon: Mail, countKey: 'email_count' },
		nats: { icon: NatsIcon, countKey: 'nats_count' },
		mqtt: { icon: MqttIcon, countKey: 'mqtt_count' },
		sqs: { icon: AwsIcon, countKey: 'sqs_count' },
		gcp: { icon: GoogleCloudIcon, countKey: 'gcp_count' },
		eventStreams: { icon: PlugZap }
	}

	function camelCaseToWords(s: string) {
		const result = s.replace(/([A-Z])/g, ' $1')
		return result.charAt(0).toUpperCase() + result.slice(1).toLowerCase()
	}
</script>

{#each triggersToDisplay as type}
	{@const { icon, countKey } = triggerTypeConfig[type]}
	{#if (!showOnlyWithCount || ((countKey && $triggersCount?.[countKey]) || 0) > 0) && !(type === 'gcp' && !$enterpriseLicense) && !(type === 'sqs' && !$enterpriseLicense) && !(type === 'kafka' && !$enterpriseLicense) && !(type === 'nats' && !$enterpriseLicense) && !(type === 'mqtt')}
		<Popover>
			<svelte:fragment slot="text">{camelCaseToWords(type)}</svelte:fragment>
			<TriggerButton
				on:click={() => {
					$selectedTrigger = type === 'eventStreams' ? 'kafka' : type
					dispatch('select')
				}}
				selected={selected &&
					($selectedTrigger === type ||
						(type === 'eventStreams' &&
							($selectedTrigger === 'kafka' ||
								$selectedTrigger === 'nats' ||
								$selectedTrigger === 'sqs' ||
								$selectedTrigger === 'mqtt' ||
								$selectedTrigger === 'gcp')))}
			>
				{#if countKey}
					<TriggerCount count={$triggersCount?.[countKey]} />
				{/if}
				<svelte:component this={icon} size={12} />
			</TriggerButton>
		</Popover>
	{/if}
{/each}
