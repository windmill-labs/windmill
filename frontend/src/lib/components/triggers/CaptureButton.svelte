<script lang="ts">
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { Button } from '$lib/components/common'
	import { Webhook, Route, Unplug, Mail, Plus } from 'lucide-svelte'
	import KafkaIcon from '$lib/components/icons/KafkaIcon.svelte'
	import { enterpriseLicense } from '$lib/stores'
	import { type CaptureTriggerKind } from '$lib/gen'
	import { createEventDispatcher } from 'svelte'
	import { captureTriggerKindToTriggerKind } from '../triggers'
	import CaptureIcon from './CaptureIcon.svelte'

	export let small = false

	let isOpen = false

	const dispatch = createEventDispatcher()

	function handleClick(kind: CaptureTriggerKind) {
		dispatch('openTriggers', {
			kind: captureTriggerKindToTriggerKind(kind),
			config: {}
		})
		isOpen = false
	}
</script>

<Popover closeButton={false} bind:open={isOpen}>
	<svelte:fragment slot="trigger">
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
					<CaptureIcon />
					<Plus size={10} class="text-red" />
				</div>
			</Button>
		{:else}
			<Button
				color="dark"
				btnClasses="rounded-l-none"
				wrapperClasses="h-full"
				nonCaptureEvent
				title="Test trigger"
			>
				<CaptureIcon />
			</Button>
		{/if}
	</svelte:fragment>
	<svelte:fragment slot="content">
		<div class="flex flex-col bg-surface">
			<button
				class="hover:bg-surface-hover p-2 transition-colors duration-150"
				on:click={() => handleClick('webhook')}
			>
				<div class="flex flex-row items-center gap-2">
					<Webhook size={16} />
					<p class="text-xs text-secondary">Webhook</p>
				</div>
			</button>
			<button
				class="hover:bg-surface-hover p-2 transition-colors duration-150"
				on:click={() => handleClick('http')}
			>
				<div class="flex flex-row items-center gap-2">
					<Route size={16} />
					<p class="text-xs text-secondary">HTTP</p>
				</div>
			</button>
			<button
				class="hover:bg-surface-hover p-2 transition-colors duration-150"
				on:click={() => handleClick('websocket')}
			>
				<div class="flex flex-row items-center gap-2">
					<Unplug size={16} />
					<p class="text-xs text-secondary">Websocket</p>
				</div>
			</button>
			<button
				class="hover:bg-surface-hover p-2 transition-colors duration-150"
				on:click={() => handleClick('email')}
			>
				<div class="flex flex-row items-center gap-2">
					<Mail size={16} />
					<p class="text-xs text-secondary">Email</p>
				</div>
			</button>
			<button
				disabled={!$enterpriseLicense}
				class="hover:bg-surface-hover p-2 transition-colors duration-150"
				on:click={() => handleClick('kafka')}
			>
				<div class="flex flex-row items-center gap-2">
					<KafkaIcon size={16} />
					<p class="text-xs text-secondary">Kafka</p>
				</div>
			</button>
		</div>
	</svelte:fragment>
</Popover>
