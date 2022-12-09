<script lang="ts">
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Popover from '$lib/components/Popover.svelte'
	import { classNames } from '$lib/utils'
	import { faBed, faRepeat, faStop, faTimesCircle } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import Icon from 'svelte-awesome'
	import { PhoneIncoming, Repeat } from 'lucide-svelte'

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
					<div class="bg-white rounded border text-gray-700 px-1 center-center">
						<Repeat size={14} />
					</div>
					<span slot="text">Retries</span>
				</Popover>
			{/if}
			{#if earlyStop}
				<Popover notClickable>
					<div class="bg-white rounded border text-gray-700 px-1">
						<Icon scale={0.8} data={faStop} />
					</div>
					<span slot="text">Early Stop/Break</span>
				</Popover>
			{/if}
			{#if sleep}
				<Popover notClickable>
					<div class="bg-white rounded border text-gray-700 px-1">
						<Icon scale={0.8} data={faBed} />
					</div>
					<span slot="text">Sleep</span>
				</Popover>
			{/if}
			{#if suspend}
				<Popover notClickable>
					<div class="bg-white rounded border text-gray-700 px-1 center-center">
						<PhoneIncoming size={12} />
					</div>
					<span slot="text">Suspend</span>
				</Popover>
			{/if}
		</div>
		<div
			class="flex justify-between items-center w-full overflow-hidden rounded-sm border border-gray-400 p-2 bg-white text-2xs module"
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
						class="absolute -top-2 right-0 rounded-full h-4 w-4 trash center-center bg-white {selected
							? ''
							: '!hidden'}"
						on:click={(event) => dispatch('delete', event)}
						><Icon
							data={faTimesCircle}
							class="text-gray-600 hover:text-red-600"
							scale={0.9}
						/></button
					>
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
