<script lang="ts">
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { twMerge } from 'tailwind-merge'
	import { getContext } from 'svelte'
	import type { PropPickerContext } from '$lib/components/prop_picker'
	import AnimatedButton from '$lib/components/common/button/AnimatedButton.svelte'
	import InputPickerInner from './InputPickerInner.svelte'
	import { ChevronDown, Plug } from 'lucide-svelte'
	import { useSvelteFlow } from '@xyflow/svelte'

	interface Props {
		selected?: boolean
		hover?: boolean
		isConnectingCandidate?: boolean
		variant?: 'default' | 'virtual'
		historyOpen?: boolean
		children?: import('svelte').Snippet<[any]>
		inputTransform?: Record<string, any> | undefined
		id: string
		bottomBarOpen?: boolean
		loopStatus?: { type: 'inside' | 'self'; flow: 'forloopflow' | 'whileloopflow' } | undefined
		onEditInput?: (moduleId: string, key: string) => void
		initial?: boolean
		onResetInitial?: () => void
	}

	let {
		selected = false,
		hover = false,
		isConnectingCandidate = false,
		variant = 'default',
		historyOpen = false,
		children,
		inputTransform,
		id,
		bottomBarOpen = $bindable(false),
		loopStatus,
		onEditInput
	}: Props = $props()

	const context = getContext<PropPickerContext>('PropPickerContext')
	const flowPropPickerConfig = context?.flowPropPickerConfig
	const MIN_WIDTH = 375
	const MIN_HEIGHT = 375

	let outputOpen = $state(false)
	let inputOpen = $state(false)

	const zoom = $derived.by(useSvelteFlow().getZoom)

	let showConnecting = $derived(
		isConnectingCandidate && $flowPropPickerConfig?.insertionMode === 'connect'
	)

	function selectConnection(value: string) {
		if ($flowPropPickerConfig?.onSelect(value)) {
			$flowPropPickerConfig?.clearFocus()
			popover?.close()
		}
	}

	let inputPopover: Popover | undefined = $state(undefined)
	let popover: Popover | undefined = $state(undefined)

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

	$effect(() => {
		bottomBarOpen = inputOpen || outputOpen || selected || hover || showConnecting
	})

	const showInput = $derived(
		variant === 'default' && !showConnecting && loopStatus?.type !== 'self'
	)

	function updatePositioning(historyOpen: boolean, zoom: number) {
		inputPopover?.updatePositioning({
			placement: 'bottom',
			gutter: 0,
			offset: { mainAxis: 3, crossAxis: 69 * zoom },
			overflowPadding: historyOpen ? 250 : 8
		})
		popover?.updatePositioning({
			placement: 'bottom',
			gutter: 0,
			offset: { mainAxis: 3, crossAxis: showInput ? -69 * zoom : 0 },
			overflowPadding: historyOpen ? 250 : 8
		})
	}

	$effect(() => {
		updatePositioning(historyOpen, zoom)
	})
</script>

<div
	class="relative h-1 w-[275px]"
	onpointerdown={(e) => {
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
						offset: { mainAxis: 3, crossAxis: 69 },
						overflowPadding: historyOpen ? 250 : 8
					}}
					usePointerDownOutside
					closeOnOutsideClick={false}
					on:click={(e) => {
						e.preventDefault()
						e.stopPropagation()
					}}
					allowFullScreen
					contentClasses="overflow-hidden resize"
					contentStyle={`width: calc(${MIN_WIDTH}px); min-width: calc(${MIN_WIDTH}px); height: calc(${MIN_HEIGHT}px); min-height: calc(${MIN_HEIGHT}px); `}
					extraProps={{ 'data-prop-picker': true }}
					closeOnOtherPopoverOpen
					disableFocusTrap
					class="flex-1 h-full"
					bind:isOpen={inputOpen}
					bind:this={inputPopover}
				>
					{#snippet trigger({ isOpen })}
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
					{/snippet}
					{#snippet content()}
						<InputPickerInner {inputTransform} {id} {onEditInput} />
					{/snippet}
				</Popover>
			{/if}
			<Popover
				floatingConfig={{
					placement: 'bottom',
					gutter: 0,
					offset: { mainAxis: 3, crossAxis: showInput ? -69 : 0 },
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
				contentClasses="overflow-hidden resize relative"
				contentStyle={`width: calc(${MIN_WIDTH}px); min-width: calc(${MIN_WIDTH}px); height: calc(${MIN_HEIGHT}px); min-height: calc(${MIN_HEIGHT}px); `}
				extraProps={{ 'data-prop-picker': true }}
				closeOnOtherPopoverOpen
				class="flex-1 h-full"
				bind:isOpen={outputOpen}
			>
				{#snippet trigger({ isOpen })}
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
				{/snippet}
				{#snippet content()}
					{@render children?.({
						allowCopy: !$flowPropPickerConfig,
						isConnecting: showConnecting,
						selectConnection
					})}
				{/snippet}
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
