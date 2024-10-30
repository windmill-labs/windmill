<script lang="ts">
	import ObjectViewer from '$lib/components/propertyPicker/ObjectViewer.svelte'
	import AnimatedButton from '$lib/components/common/button/AnimatedButton.svelte'
	import { Popup } from '$lib/components/common'
	import { Plug } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import type { PropPickerConfig } from '$lib/components/prop_picker'

	export let propPickerConfig: PropPickerConfig | undefined = undefined
	export let pickableIds: Record<string, any> | undefined = undefined
	export let id: string | undefined = undefined
</script>

{#if id}
	<AnimatedButton
		animate={propPickerConfig?.insertionMode === 'connect'}
		wrapperClasses=" h-fit w-fit center-center"
		baseRadius="9999px"
	>
		<Popup floatingConfig={{ strategy: 'fixed', placement: 'bottom-start' }}>
			<svelte:fragment slot="button">
				<button
					class={twMerge(
						'rounded-full trash center-center h-[20px] w-[20px]',
						propPickerConfig?.insertionMode !== 'connect'
							? 'outline-[1px] outline dark:outline-gray-500 outline-gray-300 duration-150 bg-blue-500 hover:bg-blue-500/80 hover:text-white text-secondary-inverse'
							: 'bg-surface text-blue-500'
					)}
				>
					<Plug class="mx-[3px]" size={12} strokeWidth={2} />
				</button>
			</svelte:fragment>
			<ObjectViewer
				json={{ [id]: pickableIds?.[id] }}
				topBrackets={false}
				pureViewer={false}
				prefix="results"
				on:select={(e) => {
					propPickerConfig?.onSelect(e.detail)
					propPickerConfig = undefined
				}}
			/>
		</Popup>
	</AnimatedButton>
{/if}
