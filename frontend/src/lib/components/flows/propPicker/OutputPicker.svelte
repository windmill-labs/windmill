<script lang="ts">
	import { ChevronDown } from 'lucide-svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { twMerge } from 'tailwind-merge'
	import { getContext, tick } from 'svelte'
	import type { PropPickerContext } from '$lib/components/prop_picker'
	import AnimatedButton from '$lib/components/common/button/AnimatedButton.svelte'

	export let zoom: number = 1
	export let selected: boolean = false
	export let hover: boolean = false
	export let isConnectingCandidate: boolean = false

	const context = getContext<PropPickerContext>('PropPickerContext')
	const flowPropPickerConfig = context?.flowPropPickerConfig
	const MIN_WIDTH = 275
	const MIN_HEIGHT = 275

	let isConnecting = false

	async function updateConnecting() {
		await tick()
		isConnecting = $flowPropPickerConfig?.insertionMode === 'connect'
	}

	$: $flowPropPickerConfig, updateConnecting()

	$: showConnecting = isConnectingCandidate && isConnecting

	function selectConnection(event: CustomEvent) {
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
	allowFullScreen
	contentClasses="overflow-hidden resize rounded-md"
	contentStyle={`width: calc(${MIN_WIDTH * zoom}px); min-width: calc(${
		MIN_WIDTH * zoom
	}px); height: calc(${MIN_HEIGHT * zoom}px); min-height: calc(${MIN_HEIGHT * zoom}px);`}
	fullScreenWidthOffset={2 * MIN_WIDTH}
	extraProps={{ 'data-prop-picker': true }}
>
	<svelte:fragment slot="trigger" let:isOpen>
		<div
			class={twMerge(
				'w-[275px] h-[4px] flex flex-row bg-surface-disabled hover:bg-surface-hover items-center justify-center cursor-pointer',
				'shadow-[inset_0_1px_5px_0_rgba(0,0,0,0.05)] rounded-b-sm',
				'group'
			)}
			data-prop-picker
			title={`${isOpen ? 'Close' : 'Open'} step output`}
		>
			<div class="absolute bottom-0 left-1/2 -translate-x-1/2 w-10 h-[14px]">
				<AnimatedButton
					animate={showConnecting}
					wrapperClasses="relative w-full h-full center-center"
					baseRadius="6px"
					marginWidth="1px"
				>
					<div
						class={twMerge(
							'w-full h-full rounded-t-md shadow-[inset_0_1px_5px_0_rgba(0,0,0,0.05)]',
							'hidden group-hover:center-center bg-surface-disabled hover:bg-surface-hover group-hover:bg-surface-hover',
							isOpen || selected || hover || showConnecting ? 'center-center' : 'hidden',
							showConnecting ? 'text-blue-500 bg-surface rounded-b-md' : 'text-secondary'
						)}
					>
						<ChevronDown
							size={12}
							class="h-fit transition-transform duration-100"
							style={`transform: rotate(${isOpen ? '180deg' : '0deg'})`}
						/>
					</div>
				</AnimatedButton>
			</div>
		</div>
	</svelte:fragment>
	<svelte:fragment slot="content">
		<slot allowCopy={!$flowPropPickerConfig} {isConnecting} {selectConnection} />
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
