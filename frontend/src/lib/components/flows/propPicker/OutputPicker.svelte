<script lang="ts">
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { twMerge } from 'tailwind-merge'
	import { getContext, tick } from 'svelte'
	import type { PropPickerContext } from '$lib/components/prop_picker'
	import AnimatedButton from '$lib/components/common/button/AnimatedButton.svelte'
	import InputPickerInner from './InputPickerInner.svelte'
	import { ChevronDown, Plug } from 'lucide-svelte'

	export let selected: boolean = false
	export let hover: boolean = false
	export let isConnectingCandidate: boolean = false
	export let variant: 'default' | 'virtual' = 'default'
	export let historyOpen: boolean = false
	export let inputTransform: Record<string, any> | undefined = undefined
	export let zoom: number = 1
	export let id: string

	const context = getContext<PropPickerContext>('PropPickerContext')
	const flowPropPickerConfig = context?.flowPropPickerConfig
	const MIN_WIDTH = 275
	const MIN_HEIGHT = 275

	let isConnecting = false
	let outputOpen = false
	let inputOpen = false

	$: width = Math.max(MIN_WIDTH * zoom, 375)
	$: height = Math.max(MIN_HEIGHT * zoom, 375)

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

	const virtualItemClasses = {
		bar: 'dark:hover:bg-[#525d6f] dark:bg-[#414958] bg-[#d7dfea] hover:bg-slate-300'
	}

	const defaultClasses = {
		bar: 'bg-surface-disabled hover:bg-surface-hover dark:bg-[#454e5f] dark:hover:bg-[#576278]'
	}

	export function toggleOpen(forceOpen: boolean = false) {
		if (popover?.isOpened() && !forceOpen) {
			popover?.close()
		} else {
			popover?.open()
		}
	}

	$: bottomBarOpen = inputOpen || outputOpen || selected || hover || showConnecting

	$: showInput = variant === 'default' && !showConnecting
</script>

<div
	class="relative h-1 w-[275px]"
	on:pointerdown={(e) => {
		e.preventDefault()
		e.stopPropagation()
	}}
>
	<!-- Invisible hover area to maintain consistent height -->
	<div class="absolute w-full h-[20px]"></div>
	<div
		class={twMerge(
			'bg-slate-200 absolute w-full',
			variant === 'virtual'
				? `${virtualItemClasses.bar} ${bottomBarOpen ? 'bg-slate-300 dark:bg-[#525d6f]' : ''}`
				: `${defaultClasses.bar} ${bottomBarOpen ? 'bg-surface-hover dark:bg-[#576278]' : ''}`,
			'shadow-[inset_0_1px_5px_0_rgba(0,0,0,0.05)] rounded-b-sm',
			'group transition-all duration-100',
			'flex flex-row items-center justify-center',
			'h-1 hover:h-[20px]',
			bottomBarOpen && 'h-[20px]'
		)}
		data-prop-picker
	>
		<div class="flex flex-row items-center justify-center w-full h-full">
			{#if showInput}
				<Popover
					floatingConfig={{
						placement: 'bottom',
						gutter: 0,
						offset: { mainAxis: 3, crossAxis: 69 * zoom },
						overflowPadding: historyOpen ? 250 : 8
					}}
					usePointerDownOutside
					closeOnOutsideClick={false}
					on:click={(e) => {
						e.preventDefault()
						e.stopPropagation()
					}}
					allowFullScreen
					contentClasses="overflow-hidden resize rounded-t-none"
					contentStyle={`width: calc(${width}px); min-width: calc(${width}px); height: calc(${height}px); min-height: calc(${height}px); `}
					extraProps={{ 'data-prop-picker': true }}
					closeOnOtherPopoverOpen
					class="flex-1 h-full"
					bind:isOpen={inputOpen}
				>
					<svelte:fragment slot="trigger">
						<button
							class={twMerge(
								'h-full center-center transition-opacity duration-150 w-full',
								bottomBarOpen ? 'opacity-100' : 'opacity-0',
								'text-2xs font-normal w-full h-full border-t-2 border-transparent',
								inputOpen ? 'border-primary' : 'hover:border-primary/20'
							)}
						>
							In
						</button>
					</svelte:fragment>
					<svelte:fragment slot="content">
						<InputPickerInner {inputTransform} {id} />
					</svelte:fragment>
				</Popover>
			{/if}
			<Popover
				floatingConfig={{
					placement: 'bottom',
					gutter: 0,
					offset: { mainAxis: 3, crossAxis: showInput ? -69 * zoom : 0 },
					overflowPadding: historyOpen ? 250 : 8
				}}
				usePointerDownOutside
				closeOnOutsideClick={false}
				on:click={(e) => {
					e.preventDefault()
					e.stopPropagation()
				}}
				bind:this={popover}
				allowFullScreen
				contentClasses="overflow-hidden resize rounded-t-none"
				contentStyle={`width: calc(${width}px); min-width: calc(${width}px); height: calc(${height}px); min-height: calc(${height}px); `}
				extraProps={{ 'data-prop-picker': true }}
				closeOnOtherPopoverOpen
				class="flex-1 h-full"
				bind:isOpen={outputOpen}
			>
				<svelte:fragment slot="trigger">
					<AnimatedButton
						animate={showConnecting}
						wrapperClasses={twMerge(
							'h-full center-center transition-opacity duration-150 w-full',
							bottomBarOpen ? 'opacity-100' : 'opacity-0'
						)}
						baseRadius="2px"
						marginWidth="1px"
					>
						<button
							class={twMerge(
								'text-2xs font-normal w-full h-full border-t-2 border-transparent',
								outputOpen ? 'border-primary' : 'hover:border-primary/20',
								showConnecting ? 'bg-surface-hover rounded-sm border-0' : ''
							)}
						>
							{#if showInput}
								Out
							{:else if showConnecting}
								<Plug size={12} class="w-full text-blue-500" />
							{:else}
								<ChevronDown size={12} class="w-full" />
							{/if}
						</button>
					</AnimatedButton>
				</svelte:fragment>
				<svelte:fragment slot="content">
					<slot allowCopy={!$flowPropPickerConfig} {isConnecting} {selectConnection} />
				</svelte:fragment>
			</Popover>
		</div>
	</div>
</div>

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
