<script lang="ts">
	import { Button } from '$lib/components/common'
	import { Webhook, Route, Unplug, Mail, Plus, Database } from 'lucide-svelte'
	import KafkaIcon from '$lib/components/icons/KafkaIcon.svelte'
	import { enterpriseLicense } from '$lib/stores'
	import { type CaptureTriggerKind } from '$lib/gen'
	import { createEventDispatcher } from 'svelte'
	import { captureTriggerKindToTriggerKind } from '../triggers'
	import CaptureIcon from './CaptureIcon.svelte'
	import NatsIcon from '../icons/NatsIcon.svelte'
	import AwsIcon from '../icons/AwsIcon.svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import MqttIcon from '../icons/MqttIcon.svelte'
	import GoogleCloudIcon from '../icons/GoogleCloudIcon.svelte'

	export let small = false

	const dispatch = createEventDispatcher()

	function handleClick(kind: CaptureTriggerKind) {
		dispatch('openTriggers', {
			kind: captureTriggerKindToTriggerKind(kind),
			config: {}
		})
	}

	$: items = [
		{
			icon: Webhook,
			displayName: 'Webhook',
			action: () => handleClick('webhook')
		},
		{
			icon: Route,
			displayName: 'HTTP',
			action: () => handleClick('http')
		},
		{
			icon: Unplug,
			displayName: 'Websocket',
			action: () => handleClick('websocket')
		},
		{
			icon: AwsIcon,
			displayName: 'SQS',
			action: () => handleClick('sqs'),
			disabled: !$enterpriseLicense
		},
		{
			icon: GoogleCloudIcon,
			displayName: 'GCP Pub/Sub',
			action: () => handleClick('gcp'),
			disabled: !$enterpriseLicense
		},
		{
			icon: MqttIcon,
			displayName: 'MQTT',
			action: () => handleClick('mqtt')
		},
		{
			icon: Database,
			displayName: 'Postgres',
			action: () => handleClick('postgres')
		},
		{
			icon: Mail,
			displayName: 'Email',
			action: () => handleClick('email')
		},
		{
			icon: KafkaIcon,
			displayName: 'Kafka',
			action: () => handleClick('kafka'),
			disabled: !$enterpriseLicense
		},
		{
			icon: NatsIcon,
			displayName: 'Nats',
			action: () => handleClick('nats'),
			disabled: !$enterpriseLicense
		}
	]
</script>

<DropdownV2 {items} placement="bottom-start" fixedHeight={false}>
	<svelte:fragment slot="buttonReplacement">
		{#if small}
			<Button
				color="light"
				size="xs"
				variant="border"
				wrapperClasses="h-full"
				nonCaptureEvent
				title="Test trigger"
			>
				<div class="flex flex-row items-center gap-1">
					<CaptureIcon variant="redDot" />
					<Plus size={10} class="text-red" />
				</div>
			</Button>
		{:else}
			<Button
				color="dark"
				btnClasses="!rounded-l-none"
				wrapperClasses="h-full"
				nonCaptureEvent
				title="Test trigger"
			>
				<CaptureIcon variant="redDot" />
			</Button>
		{/if}
	</svelte:fragment>
</DropdownV2>
