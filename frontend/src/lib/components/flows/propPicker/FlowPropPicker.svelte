<script lang="ts">
	import ObjectViewer from '$lib/components/propertyPicker/ObjectViewer.svelte'
	import AnimatedButton from '$lib/components/common/button/AnimatedButton.svelte'
	import { Popup } from '$lib/components/common'
	import { Plug } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import type { PropPickerWrapperContext } from '$lib/components/prop_picker'
	import { getContext } from 'svelte'
	import Popover from '$lib/components/Popover.svelte'

	export let json = {}
	export let prefix = ''
	export let viewOnly = false

	let popupOpen = false

	const { propPickerConfig } = getContext<PropPickerWrapperContext>('PropPickerWrapper')

	$: console.log('dbg view only', viewOnly)
	$: console.log('dbg popupOpen', popupOpen)
	$: console.log('dbg json', json)
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
		animate={$propPickerConfig?.insertionMode === 'connect' && !viewOnly}
		wrapperClasses=" h-fit w-fit center-center"
		baseRadius="9999px"
	>
		<Popup floatingConfig={{ strategy: 'fixed', placement: 'bottom-start' }}>
			<svelte:fragment slot="button" let:open>
				<Popover disablePopup={open}>
					<svelte:fragment slot="text">node outputs</svelte:fragment>
					<button
						class={twMerge(
							'rounded-full trash center-center h-[20px] w-[20px]',
							viewOnly
								? 'outline-[1px] outline dark:outline-gray-500 outline-gray-300 duration-150 bg-surface hover:bg-surface-hover text-secondary'
								: $propPickerConfig?.insertionMode == 'connect'
								? 'bg-surface text-blue-500'
								: 'outline-[1px] outline dark:outline-gray-500 outline-gray-300 duration-150 bg-blue-500 hover:bg-blue-700 text-white'
						)}
					>
						<Plug class="mx-[3px]" size={12} strokeWidth={2} />
					</button>
				</Popover>
			</svelte:fragment>
			<div data-prop-picker>
				<ObjectViewer
					{json}
					topBrackets={false}
					pureViewer={viewOnly}
					{prefix}
					on:select={(e) => {
						$propPickerConfig?.onSelect(e.detail)
						$propPickerConfig = undefined
					}}
				/>
			</div>
		</Popup>
	</AnimatedButton>
</button>
