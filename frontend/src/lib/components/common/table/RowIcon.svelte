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
		Unplug,
		Workflow
	} from 'lucide-svelte'
	import FileIcon from '$lib/components/raw_apps/FileIcon.svelte'

	interface Props {
		kind:
			| 'script'
			| 'flow'
			| 'app'
			| 'raw_app'
			| 'raw_app_file'
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
			| 'data_pipeline'
		/** For 'trigger' kind, specifies the specific trigger type (routes, schedules, etc.) */
		triggerKind?: string | undefined
		/** For 'raw_app_file' kind: the file name/path, used to pick an
		 * extension-specific icon. */
		path?: string | undefined
		size?: number
	}

	let { kind, triggerKind = undefined, path = undefined, size = 16 }: Props = $props()

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
		<BarsStaggered {size} class="text-teal-500" />
	{:else if effectiveKind === 'app' || effectiveKind === 'raw_app'}
		<LayoutDashboard {size} class="text-orange-500" />
	{:else if effectiveKind === 'raw_app_file'}
		<FileIcon name={path ?? ''} {size} />
	{:else if effectiveKind === 'script'}
		<Code2 {size} class="text-blue-500" />
	{:else if effectiveKind === 'variable'}
		<DollarSign {size} class="text-gray-400" />
	{:else if effectiveKind === 'resource'}
		<Boxes {size} class="text-gray-400" />
	{:else if effectiveKind === 'resource_type'}
		<div style="width: {size}px; height: {size}px;" class="bg-gray-100 rounded-full"></div>
	{:else if effectiveKind === 'folder'}
		<Folder {size} class="text-gray-400" />
	{:else if effectiveKind === 'schedule' || effectiveKind === 'schedules'}
		<Calendar {size} class="text-gray-400" />
	{:else if effectiveKind === 'routes'}
		<Route {size} class="text-gray-400" />
	{:else if effectiveKind === 'websockets'}
		<Unplug {size} class="text-gray-400" />
	{:else if effectiveKind === 'postgres'}
		<Database {size} class="text-gray-400" />
	{:else if effectiveKind === 'kafka'}
		<KafkaIcon {size} class="text-gray-400" />
	{:else if effectiveKind === 'nats'}
		<NatsIcon {size} class="text-gray-400" />
	{:else if effectiveKind === 'mqtt'}
		<MqttIcon {size} class="text-gray-400" />
	{:else if effectiveKind === 'sqs'}
		<AwsIcon {size} class="text-gray-400" />
	{:else if effectiveKind === 'gcp'}
		<GoogleCloudIcon {size} />
	{:else if effectiveKind === 'azure'}
		<AzureIcon {size} />
	{:else if effectiveKind === 'emails'}
		<Mail {size} class="text-gray-400" />
	{:else if effectiveKind === 'trigger'}
		<Calendar {size} class="text-gray-400" />
	{:else if effectiveKind === 'data_pipeline'}
		<Workflow {size} class="text-indigo-500" />
	{:else}
		<div style="width: {size}px;"></div>
	{/if}
</div>
