<script lang="ts">
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import KafkaIcon from '$lib/components/icons/KafkaIcon.svelte'
	import NatsIcon from '$lib/components/icons/NatsIcon.svelte'
	import MqttIcon from '$lib/components/icons/MqttIcon.svelte'
	import AwsIcon from '$lib/components/icons/AwsIcon.svelte'
	import AzureIcon from '$lib/components/icons/AzureIcon.svelte'
	import GoogleCloudIcon from '$lib/components/icons/GoogleCloudIcon.svelte'
	import {
		Boxes,
		Calendar,
		Code2,
		Database,
		DollarSign,
		Folder,
		LayoutDashboard,
		Mail,
		Route,
		Unplug
	} from 'lucide-svelte'

	interface Props {
		kind:
			| 'script'
			| 'flow'
			| 'app'
			| 'raw_app'
			| 'resource'
			| 'variable'
			| 'resource_type'
			| 'folder'
			| 'schedule'
			| 'trigger'
			| 'routes'
			| 'schedules'
			| 'websockets'
			| 'postgres'
			| 'kafka'
			| 'nats'
			| 'mqtt'
			| 'sqs'
			| 'gcp'
			| 'emails'
			| 'http_trigger'
			| 'websocket_trigger'
			| 'kafka_trigger'
			| 'nats_trigger'
			| 'postgres_trigger'
			| 'mqtt_trigger'
			| 'sqs_trigger'
			| 'gcp_trigger'
			| 'azure_trigger'
			| 'email_trigger'
		/** For 'trigger' kind, specifies the specific trigger type (routes, schedules, etc.) */
		triggerKind?: string | undefined
	}

	let { kind, triggerKind = undefined }: Props = $props()

	// Map per-kind backend names (e.g. `kafka_trigger`) to the legacy short
	// names the icon switch already handles, so we don't have to duplicate cases.
	const PER_KIND_TO_SHORT: Record<string, string> = {
		http_trigger: 'routes',
		websocket_trigger: 'websockets',
		kafka_trigger: 'kafka',
		nats_trigger: 'nats',
		postgres_trigger: 'postgres',
		mqtt_trigger: 'mqtt',
		sqs_trigger: 'sqs',
		gcp_trigger: 'gcp',
		azure_trigger: 'azure',
		email_trigger: 'emails'
	}

	let effectiveKind = $derived(
		kind === 'trigger' && triggerKind ? triggerKind : (PER_KIND_TO_SHORT[kind] ?? kind)
	)
</script>

<div class="flex justify-center items-center" title={effectiveKind}>
	{#if effectiveKind === 'flow'}
		<BarsStaggered size={16} class="text-teal-500" />
	{:else if effectiveKind === 'app' || effectiveKind === 'raw_app'}
		<LayoutDashboard size={16} class="text-orange-500" />
	{:else if effectiveKind === 'script'}
		<Code2 size={16} class="text-blue-500" />
	{:else if effectiveKind === 'variable'}
		<DollarSign size={16} class="text-gray-400" />
	{:else if effectiveKind === 'resource'}
		<Boxes size={16} class="text-gray-400" />
	{:else if effectiveKind === 'resource_type'}
		<div style="width: 16px; height: 16px;" class="bg-gray-100 rounded-full"></div>
	{:else if effectiveKind === 'folder'}
		<Folder size={16} class="text-gray-400" />
	{:else if effectiveKind === 'schedule' || effectiveKind === 'schedules'}
		<Calendar size={16} class="text-gray-400" />
	{:else if effectiveKind === 'routes'}
		<Route size={16} class="text-gray-400" />
	{:else if effectiveKind === 'websockets'}
		<Unplug size={16} class="text-gray-400" />
	{:else if effectiveKind === 'postgres'}
		<Database size={16} class="text-gray-400" />
	{:else if effectiveKind === 'kafka'}
		<KafkaIcon size={16} class="text-gray-400" />
	{:else if effectiveKind === 'nats'}
		<NatsIcon size={16} class="text-gray-400" />
	{:else if effectiveKind === 'mqtt'}
		<MqttIcon size={16} class="text-gray-400" />
	{:else if effectiveKind === 'sqs'}
		<AwsIcon size={16} class="text-gray-400" />
	{:else if effectiveKind === 'gcp'}
		<GoogleCloudIcon size={16} />
	{:else if effectiveKind === 'azure'}
		<AzureIcon size={16} />
	{:else if effectiveKind === 'emails'}
		<Mail size={16} class="text-gray-400" />
	{:else if effectiveKind === 'trigger'}
		<Calendar size={16} class="text-gray-400" />
	{:else}
		<div class="w-[16px]"></div>
	{/if}
</div>
