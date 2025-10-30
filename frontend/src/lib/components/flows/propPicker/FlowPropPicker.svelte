<script lang="ts">
	import { run, preventDefault, stopPropagation, createBubbler } from 'svelte/legacy'

	const bubble = createBubbler()
	import ObjectViewer from '$lib/components/propertyPicker/ObjectViewer.svelte'
	import AnimatedButton from '$lib/components/common/button/AnimatedButton.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import { Plug } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import type { PropPickerContext } from '$lib/components/prop_picker'
	import { getContext } from 'svelte'

	import { tick } from 'svelte'

	interface Props {
		json?: any
		prefix?: string
	}

	let { json = {}, prefix = '' }: Props = $props()

	let isConnecting = $state(false)

	const { flowPropPickerConfig } = getContext<PropPickerContext>('PropPickerContext')

	async function updateConnecting() {
		await tick()
		isConnecting = $flowPropPickerConfig?.insertionMode === 'connect'
	}

	run(() => {
		$flowPropPickerConfig
		updateConnecting()
	})
</script>

<button
	onclick={stopPropagation(
		preventDefault((e) => {
			e.preventDefault()
			e.stopPropagation()
		})
	)}
	onkeydown={stopPropagation(preventDefault(bubble('keydown')))}
	data-prop-picker
	title=""
>
	<AnimatedButton
		animate={isConnecting}
		wrapperClasses="h-[20px] w-[20px] center-center"
		baseRadius="9999px"
		marginWidth="1px"
	>
		<Popover
			floatingConfig={{ strategy: 'fixed', placement: 'bottom-start' }}
			closeOnOtherPopoverOpen={true}
		>
			{#snippet trigger({ isOpen })}
				<Tooltip disablePopup={isOpen}>
					{#snippet text()}
						node outputs
					{/snippet}
					<button
						class={twMerge(
							'rounded-full center-center h-[18px] w-[18px]',
							isConnecting
								? 'bg-surface text-blue-500'
								: 'outline-[1px] outline dark:outline-gray-500 outline-gray-300 duration-0 bg-blue-500 hover:bg-blue-700 text-white'
						)}
					>
						<Plug size={12} strokeWidth={2} />
					</button>
				</Tooltip>
			{/snippet}
			{#snippet content()}
				<div class="p-4 max-h-[50vh] overflow-y-auto" data-prop-picker>
					<ObjectViewer
						{json}
						topBrackets={false}
						pureViewer={false}
						{prefix}
						on:select={({ detail }) => {
							if ($flowPropPickerConfig?.onSelect(detail)) {
								$flowPropPickerConfig?.clearFocus()
							}
						}}
						allowCopy={!$flowPropPickerConfig}
					/>
				</div>
			{/snippet}
		</Popover>
	</AnimatedButton>
</button>
