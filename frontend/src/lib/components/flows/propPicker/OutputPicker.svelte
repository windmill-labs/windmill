<script lang="ts">
	import { ChevronDown } from 'lucide-svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { twMerge } from 'tailwind-merge'
	import { getContext, tick } from 'svelte'
	import type { PropPickerContext } from '$lib/components/prop_picker'

	export let zoom: number = 1
	export let selected: boolean = false
	export let hover: boolean = false
	export let isConnectingCandidate: boolean = false

	const context = getContext<PropPickerContext>('PropPickerContext')
	const flowPropPickerConfig = context?.flowPropPickerConfig

	let isConnecting = false

	async function updateConnecting() {
		await tick()
		isConnecting = $flowPropPickerConfig?.insertionMode === 'connect'
	}

	$: $flowPropPickerConfig, updateConnecting()

	$: showConnecting = isConnectingCandidate && isConnecting

	function select(event: CustomEvent) {
		if ($flowPropPickerConfig?.onSelect(event.detail)) {
			$flowPropPickerConfig?.clearFocus()
			popover?.close()
		}
	}

	let popover: Popover | undefined = undefined
</script>

<Popover
	placement="bottom"
	usePointerDownOutside
	closeOnOutsideClick={false}
	on:click={(e) => {
		e.preventDefault()
		e.stopPropagation()
	}}
	bind:this={popover}
>
	<svelte:fragment slot="trigger" let:isOpen>
		<div
			class={twMerge(
				'w-[275px] h-[8px] flex flex-row bg-surface-disabled hover:bg-surface-hover items-center justify-center cursor-pointer',
				'shadow-[inset_0_1px_5px_0_rgba(0,0,0,0.05)] rounded-b-sm',
				'group relative'
			)}
			data-prop-picker
		>
			<div
				class={twMerge(
					'absolute bottom-0 left-1/2 -translate-x-1/2 w-10 h-[16px] rounded-md shadow-[inset_0_1px_5px_0_rgba(0,0,0,0.05)]',
					'hidden group-hover:center-center bg-surface-disabled hover:bg-surface-hover group-hover:bg-surface-hover',
					isOpen || selected || hover || showConnecting ? 'center-center' : 'hidden',
					showConnecting ? 'text-white' : 'text-secondary',
					showConnecting && 'gradient-animation'
				)}
			>
				<ChevronDown
					size={14}
					class="h-fit transition-transform duration-100"
					style={`transform: rotate(${isOpen ? '180deg' : '0deg'})`}
				/>
			</div>
		</div>
	</svelte:fragment>
	<svelte:fragment slot="content">
		<div
			class={twMerge(
				'overflow-hidden resize rounded-sm',
				selected && 'outline outline-offset-0  outline-2  outline-slate-500 dark:outline-gray-400'
			)}
			style={`width: calc(${275 * zoom}px); min-width: calc(${275 * zoom}px); height: calc(${
				180 * zoom
			}px); min-height: calc(${180 * zoom}px);`}
			data-prop-picker
		>
			<slot allowCopy={!$flowPropPickerConfig} {isConnecting} {select} />
		</div>
	</svelte:fragment>
</Popover>

<style>
	@keyframes moveGradient {
		0% {
			background-position: 0% 50%;
		}
		100% {
			background-position: 100% 50%;
		}
	}

	.gradient-animation {
		background: linear-gradient(
			90deg,
			rgb(59 130 246) 33%,
			rgb(255 255 255) 50%,
			rgb(59 130 246) 66%
		);
		background-size: 300% 100%;
		animation: moveGradient 2s linear infinite;
	}
</style>
