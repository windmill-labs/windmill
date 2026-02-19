<script lang="ts">
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import KafkaIcon from '$lib/components/icons/KafkaIcon.svelte'
	import NatsIcon from '$lib/components/icons/NatsIcon.svelte'
	import MqttIcon from '$lib/components/icons/MqttIcon.svelte'
	import AwsIcon from '$lib/components/icons/AwsIcon.svelte'
	import GoogleCloudIcon from '$lib/components/icons/GoogleCloudIcon.svelte'
	import {
		Boxes,
		Calendar,
		Code2,
		Database,
		DollarSign,
		Folder,
		LayoutDashboard,
		Route,
		Unplug
	} from 'lucide-svelte'

	export let kind:
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

	/** For 'trigger' kind, specifies the specific trigger type (routes, schedules, etc.) */
	export let triggerKind: string | undefined = undefined

	// Use triggerKind if kind is 'trigger' and triggerKind is provided
	$: effectiveKind = kind === 'trigger' && triggerKind ? triggerKind : kind
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
	{:else if effectiveKind === 'trigger'}
		<Calendar size={16} class="text-gray-400" />
	{:else}
		<div class="w-[16px]"></div>
	{/if}
</div>
