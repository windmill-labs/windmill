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
	export let variant: 'default' | 'virtual' = 'default'
	export let historyOpen: boolean = false

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

	$: width = Math.max(MIN_WIDTH * zoom, 375)
	$: height = Math.max(MIN_HEIGHT * zoom, 375)

	const virtualItemClasses = {
		bar: 'dark:hover:bg-[#525d6f] dark:bg-[#414958] bg-[#d7dfea] hover:bg-slate-300'
	}

	const defaultClasses = {
		bar: 'bg-surface-disabled hover:bg-surface-hover dark:bg-[#454e5f] dark:hover:bg-[#576278]'
	}

	export function toggleOpen() {
		if (popover?.isOpened()) {
			popover?.close()
		} else {
			popover?.open()
		}
	}
</script>

<!-- svelte-ignore element_invalid_self_closing_tag -->
<Popover
	floatingConfig={{
		placement: 'bottom',
		gutter: 24, // hack to make offset effective, see https://github.com/melt-ui/melt-ui/issues/528
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
	contentClasses="overflow-hidden resize rounded-md"
	contentStyle={`width: calc(${width}px); min-width: calc(${width}px); height: calc(${height}px); min-height: calc(${height}px);`}
	extraProps={{ 'data-prop-picker': true }}
	closeOnOtherPopoverOpen
	class="outline-none"
>
	<svelte:fragment slot="trigger" let:isOpen>
		<div
			class="relative h-1"
			on:pointerdown={(e) => {
				e.preventDefault()
				e.stopPropagation()
			}}
		>
			<!-- Invisible hover area to maintain consistent height -->
			<div class="absolute w-[275px] h-[16px]" />
			<div
				class={twMerge(
					'bg-slate-200 absolute w-[275px]',
					variant === 'virtual'
						? `${virtualItemClasses.bar} ${isOpen || selected || hover || showConnecting ? 'bg-slate-300 dark:bg-[#525d6f]' : ''}`
						: `${defaultClasses.bar} ${isOpen || selected || hover || showConnecting ? 'bg-surface-hover dark:bg-[#576278]' : ''}`,
					'shadow-[inset_0_1px_5px_0_rgba(0,0,0,0.05)] rounded-b-sm',
					'group transition-all duration-100',
					'flex flex-row items-center justify-center',
					'h-1 hover:h-[16px]',
					(isOpen || selected || hover || showConnecting) && 'h-[16px]',
					showConnecting && 'text-blue-500 bg-surface'
				)}
				data-prop-picker
				title={`${isOpen ? 'Close' : 'Open'} step output`}
			>
				<AnimatedButton
					animate={showConnecting}
					wrapperClasses={twMerge(
						'relative w-10 h-full center-center transition-opacity duration-150',
						isOpen || selected || hover || showConnecting ? 'opacity-100' : 'opacity-0',
						'group-hover:opacity-100'
					)}
					baseRadius="6px"
					marginWidth="1px"
				>
					<p class="text-xs">O</p>
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
