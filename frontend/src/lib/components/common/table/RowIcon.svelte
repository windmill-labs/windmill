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
		Minus,
		Plus,
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
		/** Change-operation overlay: `add` renders a + in the icon's own color;
		 * `delete` turns the whole glyph red and renders a red −. */
		op?: 'add' | 'delete' | undefined
	}

	let {
		kind,
		triggerKind = undefined,
		path = undefined,
		size = 16,
		op = undefined
	}: Props = $props()

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

	// Per-kind default color (mirrors the branch colors below) so the op overlay
	// can match the glyph, and a delete can recolor both to red in one place.
	const KIND_COLOR: Record<string, string> = {
		flow: 'text-teal-500',
		app: 'text-orange-500',
		raw_app: 'text-orange-500',
		script: 'text-blue-500',
		data_pipeline: 'text-indigo-500'
	}
	let glyphColor = $derived(
		op === 'delete'
			? 'text-red-500 dark:text-red-400'
			: (KIND_COLOR[effectiveKind] ?? 'text-gray-400')
	)
	let markSize = $derived(Math.max(8, Math.round(size * 0.7)))
</script>

<div class="relative flex justify-center items-center" title={effectiveKind}>
	{#if effectiveKind === 'flow'}
		<BarsStaggered {size} class={glyphColor} />
	{:else if effectiveKind === 'app' || effectiveKind === 'raw_app'}
		<LayoutDashboard {size} class={glyphColor} />
	{:else if effectiveKind === 'raw_app_file'}
		<FileIcon name={path ?? ''} {size} />
	{:else if effectiveKind === 'script'}
		<Code2 {size} class={glyphColor} />
	{:else if effectiveKind === 'variable'}
		<DollarSign {size} class={glyphColor} />
	{:else if effectiveKind === 'resource'}
		<Boxes {size} class={glyphColor} />
	{:else if effectiveKind === 'resource_type'}
		<div style="width: {size}px; height: {size}px;" class="bg-gray-100 rounded-full"></div>
	{:else if effectiveKind === 'folder'}
		<Folder {size} class={glyphColor} />
	{:else if effectiveKind === 'schedule' || effectiveKind === 'schedules'}
		<Calendar {size} class={glyphColor} />
	{:else if effectiveKind === 'routes'}
		<Route {size} class={glyphColor} />
	{:else if effectiveKind === 'websockets'}
		<Unplug {size} class={glyphColor} />
	{:else if effectiveKind === 'postgres'}
		<Database {size} class={glyphColor} />
	{:else if effectiveKind === 'kafka'}
		<KafkaIcon {size} class={glyphColor} />
	{:else if effectiveKind === 'nats'}
		<NatsIcon {size} class={glyphColor} />
	{:else if effectiveKind === 'mqtt'}
		<MqttIcon {size} class={glyphColor} />
	{:else if effectiveKind === 'sqs'}
		<AwsIcon {size} class={glyphColor} />
	{:else if effectiveKind === 'gcp'}
		<GoogleCloudIcon {size} />
	{:else if effectiveKind === 'azure'}
		<AzureIcon {size} />
	{:else if effectiveKind === 'emails'}
		<Mail {size} class={glyphColor} />
	{:else if effectiveKind === 'trigger'}
		<Calendar {size} class={glyphColor} />
	{:else if effectiveKind === 'data_pipeline'}
		<Workflow {size} class={glyphColor} />
	{:else}
		<div style="width: {size}px;"></div>
	{/if}
	{#if op}
		<!-- Change-op mark in the icon's own color (red for delete), on a bg-surface
		     ring so it stays legible over the glyph. -->
		<span
			class="absolute -bottom-1 -right-1 flex items-center justify-center rounded-full bg-surface {glyphColor}"
		>
			{#if op === 'add'}
				<Plus size={markSize} strokeWidth={3.5} />
			{:else}
				<Minus size={markSize} strokeWidth={3.5} />
			{/if}
		</span>
	{/if}
</div>
