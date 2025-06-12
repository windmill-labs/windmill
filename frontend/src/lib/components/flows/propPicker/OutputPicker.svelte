<script lang="ts">
	import { ChevronDown } from 'lucide-svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { twMerge } from 'tailwind-merge'
	import { getContext } from 'svelte'
	import type { PropPickerContext } from '$lib/components/prop_picker'
	import AnimatedButton from '$lib/components/common/button/AnimatedButton.svelte'

	interface Props {
		selected?: boolean
		hover?: boolean
		isConnectingCandidate?: boolean
		variant?: 'default' | 'virtual'
		historyOpen?: boolean
		children?: import('svelte').Snippet<[any]>
	}

	let {
		selected = false,
		hover = false,
		isConnectingCandidate = false,
		variant = 'default',
		historyOpen = false,
		children
	}: Props = $props()

	const context = getContext<PropPickerContext>('PropPickerContext')
	const flowPropPickerConfig = context?.flowPropPickerConfig
	const MIN_WIDTH = 375
	const MIN_HEIGHT = 375

	let showConnecting = $derived(
		isConnectingCandidate && $flowPropPickerConfig?.insertionMode === 'connect'
	)

	function selectConnection(value: string) {
		if ($flowPropPickerConfig?.onSelect(value)) {
			$flowPropPickerConfig?.clearFocus()
			popover?.close()
		}
	}

	let popover: Popover | undefined = $state(undefined)

	const virtualItemClasses = {
		bar: 'dark:hover:bg-[#525d6f] dark:bg-[#414958] bg-[#d7dfea]  hover:bg-slate-300',
		handle:
			'dark:group-hover:bg-[#525d6f] dark:hover:bg-[#525d6f] dark:bg-[#414958] bg-[#d7dfea] hover:bg-slate-300 group-hover:bg-slate-300'
	}

	const defaultClasses = {
		bar: 'bg-surface-disabled hover:bg-surface-hover dark:bg-[#454e5f] dark:hover:bg-[#576278]',
		handle:
			'group-hover:bg-surface-hover hover:bg-surface-hover bg-surface-disabled dark:bg-[#454e5f] dark:hover:bg-[#576278] dark:group-hover:bg-[#576278]'
	}

	export function toggleOpen() {
		if (popover?.isOpened()) {
			popover?.close()
		} else {
			popover?.open()
		}
	}
</script>

<Popover
	floatingConfig={{
		placement: 'bottom',
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
	contentStyle={`width: calc(${MIN_WIDTH}px); min-width: calc(${MIN_WIDTH}px); height: calc(${MIN_HEIGHT}px); min-height: calc(${MIN_HEIGHT}px);`}
	extraProps={{ 'data-prop-picker': true }}
	closeOnOtherPopoverOpen
	class="outline-none"
>
	{#snippet trigger({ isOpen })}
		<div
			class={twMerge(
				'bg-slate-200',
				`w-[275px] h-[4px] flex flex-row items-center justify-center cursor-pointer`,
				variant === 'virtual' ? virtualItemClasses.bar : defaultClasses.bar,
				'shadow-[inset_0_1px_5px_0_rgba(0,0,0,0.05)] rounded-b-sm',
				'group'
			)}
			onpointerdown={(e) => {
				e.preventDefault()
				e.stopPropagation()
			}}
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
							`hidden group-hover:center-center`,
							variant === 'virtual' ? virtualItemClasses.handle : defaultClasses.handle,
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
	{/snippet}
	{#snippet content()}
		{@render children?.({
			allowCopy: !$flowPropPickerConfig,
			isConnecting: showConnecting,
			selectConnection
		})}
	{/snippet}
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
