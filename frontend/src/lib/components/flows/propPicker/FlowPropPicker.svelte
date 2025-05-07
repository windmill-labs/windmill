<script lang="ts">
	import ObjectViewer from '$lib/components/propertyPicker/ObjectViewer.svelte'
	import AnimatedButton from '$lib/components/common/button/AnimatedButton.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import { Plug } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import type { PropPickerContext } from '$lib/components/prop_picker'
	import { getContext } from 'svelte'

	import { tick } from 'svelte'

	export let json = {}
	export let prefix = ''

	let isConnecting = false

	const { flowPropPickerConfig } = getContext<PropPickerContext>('PropPickerContext')

	async function updateConnecting() {
		await tick()
		isConnecting = $flowPropPickerConfig?.insertionMode === 'connect'
	}

	$: $flowPropPickerConfig, updateConnecting()
</script>

<button
	on:click|preventDefault|stopPropagation={(e) => {
		e.preventDefault()
		e.stopPropagation()
	}}
	on:keydown|preventDefault|stopPropagation
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
			<svelte:fragment slot="trigger" let:isOpen>
				<Tooltip disablePopup={isOpen}>
					<svelte:fragment slot="text">node outputs</svelte:fragment>
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
			</svelte:fragment>
			<svelte:fragment slot="content">
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
			</svelte:fragment>
		</Popover>
	</AnimatedButton>
</button>
