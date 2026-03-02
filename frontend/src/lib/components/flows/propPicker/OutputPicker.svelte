<script lang="ts">
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { twMerge } from 'tailwind-merge'
	import { getContext, onMount } from 'svelte'
	import type { PropPickerContext } from '$lib/components/prop_picker'
	import AnimatedButton from '$lib/components/common/button/AnimatedButton.svelte'
	import InputPickerInner from './InputPickerInner.svelte'
	import { ChevronDown, Plug } from 'lucide-svelte'
	import { useSvelteFlow } from '@xyflow/svelte'
	import type { FlowEditorContext } from '../types'
	import Button from '$lib/components/common/button/Button.svelte'

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

	let showConnecting = $derived(isConnectingCandidate && $flowPropPickerConfig != undefined)

	function selectConnection(value: string) {
		if ($flowPropPickerConfig?.onSelect(value)) {
			$flowPropPickerConfig?.clearFocus()
			popover?.close()
		}
	}

	let inputPopover: Popover | undefined = $state(undefined)
	let popover: Popover | undefined = $state(undefined)

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
			overflowPadding: historyOpen ? 250 : 8,
			flip: false
		})
		popover?.updatePositioning({
			placement: 'bottom',
			gutter: 0,
			offset: { mainAxis: 3, crossAxis: showInput ? -69 * zoom : 0 },
			overflowPadding: historyOpen ? 250 : 8,
			flip: false
		})
	}

	$effect(() => {
		updatePositioning(historyOpen, zoom)
	})

	onMount(() => {
		let { outputPickerOpenFns } = getContext<FlowEditorContext>('FlowEditorContext') || {}
		if (outputPickerOpenFns) {
			outputPickerOpenFns[id] = () => {
				outputOpen = true
			}
			return () => {
				delete outputPickerOpenFns[id]
			}
		}
	})
</script>

<div
	class="relative h-0 w-[275px] -z-10"
	onpointerdown={(e) => {
		e.preventDefault()
		e.stopPropagation()
	}}
>
	<!-- Invisible hover area to maintain consistent height -->
	<div class="absolute w-full h-[20px]"></div>
	<div
		class={twMerge(
			'absolute w-full top-1',
			variant === 'virtual' ? `` : ``,
			'group transition-all duration-100',
			'flex flex-row items-center justify-center',
			'h-0 hover:h-[18px]',
			bottomBarOpen && 'h-[18px]'
		)}
		data-prop-picker
	>
		<div class="flex flex-row gap-2 items-center justify-center w-full h-full">
			{#if showInput}
				<Popover
					enableFlyTransition
					floatingConfig={{
						placement: 'bottom',
						gutter: 0,
						offset: { mainAxis: 3, crossAxis: 69 },
						overflowPadding: historyOpen ? 250 : 8,
						flip: false
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
					portal="#flow-graph-v2"
				>
					{#snippet trigger({ isOpen })}
						<Button
							variant="default"
							selected={inputOpen}
							wrapperClasses={twMerge(
								'w-full h-full transition-colors rounded-b-md',
								bottomBarOpen ? 'opacity-100' : 'opacity-0',
								inputOpen ? 'bg-surface-secondary' : 'bg-surface-tertiary'
							)}
							btnClasses="font-normal"
							endIcon={{ icon: ChevronDown }}
							size="xs3"
						>
							In
						</Button>
					{/snippet}
					{#snippet content()}
						<InputPickerInner {inputTransform} {id} {onEditInput} />
					{/snippet}
				</Popover>
			{/if}

			<Popover
				enableFlyTransition
				floatingConfig={{
					placement: 'bottom',
					gutter: 0,
					offset: { mainAxis: 3, crossAxis: showInput ? -69 : 0 },
					overflowPadding: historyOpen ? 250 : 8,
					flip: false
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
				portal="#flow-graph-v2"
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
						<Button
							variant="default"
							selected={outputOpen}
							wrapperClasses={twMerge(
								'w-full h-full rounded-b-md transition-colors',
								outputOpen ? 'bg-surface-secondary' : 'bg-surface-tertiary'
							)}
							btnClasses="font-normal"
							endIcon={showInput ? { icon: ChevronDown } : undefined}
							size="xs3"
						>
							{#if showInput}
								Out
							{:else if showConnecting}
								<Plug size={12} class="w-full text-blue-500" />
							{:else}
								<ChevronDown size={12} class="w-full" />
							{/if}
						</Button>
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
