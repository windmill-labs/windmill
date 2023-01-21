<script lang="ts">
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Popover from '$lib/components/Popover.svelte'
	import { classNames } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import { fade } from 'svelte/transition'
	import { Bed, PhoneIncoming, Repeat, Square, X } from 'lucide-svelte'

	export let isFirst: boolean = false
	export let isLast: boolean = false
	export let hasLine: boolean = true
	export let selected: boolean = false
	export let deletable: boolean = false
	export let retry: boolean = false
	export let earlyStop: boolean = false
	export let suspend: boolean = false
	export let sleep: boolean = false
	export let bold: boolean = false
	export let id: string | undefined = undefined
	export let label: string

	const margin = isLast ? '' : isFirst ? 'mb-0.5' : 'my-0.5'
	const dispatch = createEventDispatcher()
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<div class="flex relative" on:click>
	<div
		class={classNames(
			'flex pl-6 ml-0.5',
			hasLine ? 'line' : '',
			isFirst ? 'justify-center items-start' : 'justify-center items-center'
		)}
	/>
	<div
		class={classNames(
			'w-full flex overflow-hidden rounded-sm cursor-pointer mr-2',
			selected ? 'outline outline-offset-1 outline-2  outline-gray-600' : '',
			margin
		)}
	>
		<div class="absolute text-sm right-12 -bottom-3 flex flex-row gap-1 z-20">
			{#if retry}
				<Popover notClickable>
					<div
						transition:fade={{ duration: 200 }}
						class="center-center bg-white rounded border border-gray-400 text-gray-700 px-1 py-0.5"
					>
						<Repeat size={14} />
					</div>
					<svelte:fragment slot="text">Retries</svelte:fragment>
				</Popover>
			{/if}
			{#if earlyStop}
				<Popover notClickable>
					<div
						transition:fade={{ duration: 200 }}
						class="center-center bg-white rounded border border-gray-400 text-gray-700 px-1 py-0.5"
					>
						<Square size={14} />
					</div>
					<svelte:fragment slot="text">Early stop/break</svelte:fragment>
				</Popover>
			{/if}
			{#if suspend}
				<Popover notClickable>
					<div
						transition:fade={{ duration: 200 }}
						class="center-center bg-white rounded border border-gray-400 text-gray-700 px-1 py-0.5"
					>
						<PhoneIncoming size={14} />
					</div>
					<svelte:fragment slot="text">Suspend</svelte:fragment>
				</Popover>
			{/if}
			{#if sleep}
				<Popover notClickable>
					<div
						transition:fade={{ duration: 200 }}
						class="center-center bg-white rounded border border-gray-400 text-gray-700 px-1 py-0.5"
					>
						<Bed size={14} />
					</div>
					<svelte:fragment slot="text">Sleep</svelte:fragment>
				</Popover>
			{/if}
		</div>
		<div
			class="flex justify-between items-center w-full overflow-hidden rounded-sm 
			border border-gray-400 p-2 bg-white text-2xs module"
		>
			{#if $$slots.icon}
				<slot name="icon" />
				<span class="mr-2" />
			{/if}
			<div class="flex-1 truncate" class:font-bold={bold}>{label}</div>
			<div class="flex items-center space-x-2">
				{#if id}
					<Badge color="indigo">{id}</Badge>
				{/if}
				{#if deletable}
					<button
						class="absolute -top-2 right-0 rounded-full h-4 w-4 trash center-center 
						border-[1.5px] border-gray-700 bg-white duration-150 hover:bg-red-400 hover:text-white 
						hover:border-red-700 {selected ? '' : '!hidden'}"
						on:click|preventDefault|stopPropagation={(event) => dispatch('delete', event)}
					>
						<X size={12} strokeWidth={2} />
					</button>
				{/if}
			</div>
		</div>
	</div>
</div>

<style>
	.module:hover .trash {
		display: flex !important;
	}
	.line {
		background: repeating-linear-gradient(to bottom, transparent 0 4px, rgb(120, 120, 120) 4px 8px)
			50%/1px 100% no-repeat;
	}
</style>
