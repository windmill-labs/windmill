<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import type { FlowModule } from '$lib/gen'
	import { classNames } from '$lib/utils'
	import { faBed, faCodeBranch, faSave, faStop } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import Icon from 'svelte-awesome'
	import { Bed, PhoneIncoming, Repeat, Square } from 'lucide-svelte'
	import Popover from '../../Popover.svelte'

	export let module: FlowModule

	const dispatch = createEventDispatcher()

	let width = 0

	$: moduleRetry = module.retry?.constant || module.retry?.exponential
</script>

<div class="flex flex-row space-x-2" bind:clientWidth={width}>
	{#if module.value.type === 'script' || module.value.type === 'rawscript'}
		<Popover
			class="center-center rounded border p-2 duration-200
			{moduleRetry
				? 'bg-blue-100 text-blue-800 border-blue-300 hover:bg-blue-200'
				: 'bg-white text-gray-800 border-gray-300 hover:bg-gray-100'}"
			on:click={() => dispatch('toggleRetry')}
		>
			<Repeat size={14} />
			<svelte:fragment slot="text">Retries</svelte:fragment>
		</Popover>
		<Popover
			class="center-center rounded border p-2 duration-200
			{module.stop_after_if
				? 'bg-blue-100 text-blue-800 border-blue-300 hover:bg-blue-200'
				: 'bg-white text-gray-800 border-gray-300 hover:bg-gray-100'}"
			on:click={() => dispatch('toggleStopAfterIf')}
		>
			<Square size={14} />
			<svelte:fragment slot="text">Early stop/break</svelte:fragment>
		</Popover>
		<Popover
			class="center-center rounded border p-2 duration-200
			{module.suspend
				? 'bg-blue-100 text-blue-800 border-blue-300 hover:bg-blue-200'
				: 'bg-white text-gray-800 border-gray-300 hover:bg-gray-100'}"
			on:click={() => dispatch('toggleSuspend')}
		>
			<PhoneIncoming size={14} />
			<svelte:fragment slot="text">Suspend</svelte:fragment>
		</Popover>
		<Popover
			class="center-center rounded border p-2 duration-200
			{module.sleep
				? 'bg-blue-100 text-blue-800 border-blue-300 hover:bg-blue-200'
				: 'bg-white text-gray-800 border-gray-300 hover:bg-gray-100'}"
			on:click={() => dispatch('toggleSleep')}
		>
			<Bed size={14} />
			<svelte:fragment slot="text">Sleep</svelte:fragment>
		</Popover>
	{/if}
	{#if module.value.type === 'script'}
		<div class="w-2" />
		<Button
			size="xs"
			color="light"
			variant="border"
			on:click={() => dispatch('fork')}
			startIcon={{ icon: faCodeBranch }}
			iconOnly={false}
		>
			Fork
		</Button>
	{/if}

	{#if module.value.type === 'rawscript'}
		<Button
			size="xs"
			color="light"
			variant="border"
			startIcon={{ icon: faSave }}
			on:click={() => dispatch('createScriptFromInlineScript')}
			iconOnly={false}
		>
			Save to workspace
		</Button>
	{/if}
</div>
