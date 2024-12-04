<script lang="ts">
	import { type CaptureTriggerKind } from '$lib/gen'
	import CaptureTable from '$lib/components/triggers/CaptureTable.svelte'
	import { Webhook, Route, Unplug, Mail } from 'lucide-svelte'
	import KafkaIcon from '$lib/components/icons/KafkaIcon.svelte'
	import { enterpriseLicense } from '$lib/stores'

	let selected: CaptureTriggerKind = 'webhook'

	export let isFlow = false
	export let path: string
</script>

<div class="w-[650px] flex flex-row divide-x overflow-hidden">
	<div class="w-40">
		<div class="flex flex-col bg-surface-secondary">
			<button
				class={`${selected === 'webhook' ? 'bg-surface' : ''} p-2`}
				on:click={() => (selected = 'webhook')}
			>
				<div class="flex flex-row items-center gap-2">
					<Webhook size={16} />
					<p class="text-xs text-secondary">Webhook</p>
				</div>
			</button>
			<button
				class={`${selected === 'http' ? 'bg-surface' : ''} p-2`}
				on:click={() => (selected = 'http')}
			>
				<div class="flex flex-row items-center gap-2">
					<Route size={16} />
					<p class="text-xs text-secondary">HTTP</p>
				</div>
			</button>
			<button
				class={`${selected === 'websocket' ? 'bg-surface' : ''} p-2`}
				on:click={() => (selected = 'websocket')}
			>
				<div class="flex flex-row items-center gap-2">
					<Unplug size={16} />
					<p class="text-xs text-secondary">Websocket</p>
				</div>
			</button>
			<button
				class={`${selected === 'email' ? 'bg-surface' : ''} p-2`}
				on:click={() => (selected = 'email')}
			>
				<div class="flex flex-row items-center gap-2">
					<Mail size={16} />
					<p class="text-xs text-secondary">Email</p>
				</div>
			</button>
			<button
				disabled={!$enterpriseLicense}
				class={`${selected === 'kafka' ? 'bg-surface' : ''} p-2`}
				on:click={() => (selected = 'kafka')}
			>
				<div class="flex flex-row items-center gap-2">
					<KafkaIcon size={16} />
					<p class="text-xs text-secondary">Kafka</p>
				</div>
			</button>
		</div>
	</div>
	<div class="grow p-2">
		<CaptureTable captureType={selected} {isFlow} {path} />
	</div>
</div>
