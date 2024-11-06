<script lang="ts">
	import ObjectViewer from '$lib/components/propertyPicker/ObjectViewer.svelte'
	import AnimatedButton from '$lib/components/common/button/AnimatedButton.svelte'
	import { Popup } from '$lib/components/common'
	import { Plug } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import type { PropPickerContext } from '$lib/components/prop_picker'
	import { getContext } from 'svelte'
	import Popover from '$lib/components/Popover.svelte'
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
>
	<AnimatedButton
		animate={isConnecting}
		wrapperClasses="h-[20px] w-[20px] "
		baseRadius="9999px"
		marginWidth="1px"
	>
		<Popup floatingConfig={{ strategy: 'fixed', placement: 'bottom-start' }}>
			<svelte:fragment slot="button" let:open>
				<Popover disablePopup={open}>
					<svelte:fragment slot="text">node outputs</svelte:fragment>
					<button
						class={twMerge(
							'rounded-full center-center h-[18px] w-[18px]',
							isConnecting
								? 'bg-surface text-blue-500'
								: 'outline-[1px] outline dark:outline-gray-500 outline-gray-300 duration-150 bg-blue-500 hover:bg-blue-700 text-white'
						)}
					>
						<Plug size={12} strokeWidth={2} />
					</button>
				</Popover>
			</svelte:fragment>
			<div data-prop-picker>
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
		</Popup>
	</AnimatedButton>
</button>
