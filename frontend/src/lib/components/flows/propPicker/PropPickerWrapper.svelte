<script lang="ts">
	import PropPicker from '$lib/components/propertyPicker/PropPicker.svelte'
	import PropPickerResult from '$lib/components/propertyPicker/PropPickerResult.svelte'
	import { clickOutside } from '$lib/utils'
	import { createEventDispatcher, getContext } from 'svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import type { PickableProperties } from '../previousResults'
	import { twMerge } from 'tailwind-merge'
	import AnimatedButton from '$lib/components/common/button/AnimatedButton.svelte'
	import type { PropPickerWrapperContext } from '$lib/components/prop_picker'
	export let pickableProperties: PickableProperties | undefined
	export let result: any = undefined
	export let extraResults: any = undefined
	export let flow_input: any = undefined
	export let error: boolean = false
	export let displayContext = true
	export let notSelectable = false
	export let noPadding: boolean = false

	const dispatch = createEventDispatcher()

	const { propPickerConfig } = getContext<PropPickerWrapperContext>('PropPickerWrapper')

	async function getPropPickerElements(): Promise<HTMLElement[]> {
		return Array.from(
			document.querySelectorAll('[data-prop-picker], [data-prop-picker] *')
		) as HTMLElement[]
	}
</script>

<div
	class="h-full w-full"
	data-prop-picker-root
	use:clickOutside={{ capture: true, exclude: getPropPickerElements }}
	on:click_outside={() => {
		propPickerConfig.set(undefined)
	}}
>
	<Splitpanes class="splitpanes-remove-splitter">
		<Pane
			minSize={20}
			size={60}
			class={twMerge('relative !transition-none ', noPadding ? '' : 'p-2')}
		>
			<slot />
		</Pane>
		<Pane minSize={20} size={40} class="!transition-none z-1000 ml-[-1px]">
			<AnimatedButton
				animate={$propPickerConfig?.insertionMode == 'connect'}
				baseRadius="4px"
				wrapperClasses="h-full w-full pt-2"
				marginWidth="4px"
				ringColor={$propPickerConfig?.insertionMode == 'insert' ? '#3b82f6' : 'transparent'}
				animationDuration="1s"
			>
				{#if result}
					<PropPickerResult
						{result}
						{extraResults}
						{flow_input}
						allowCopy={!notSelectable && !$propPickerConfig}
						on:select={({ detail }) => {
							dispatch('select', detail)
							if ($propPickerConfig?.onSelect(detail)) {
								propPickerConfig.set(undefined)
							}
						}}
					/>
				{:else if pickableProperties}
					<PropPicker
						{displayContext}
						{error}
						{pickableProperties}
						{notSelectable}
						allowCopy={!notSelectable && !$propPickerConfig}
						on:select={({ detail }) => {
							dispatch('select', detail)
							if ($propPickerConfig?.onSelect(detail)) {
								propPickerConfig.set(undefined)
							}
						}}
					/>
				{/if}
			</AnimatedButton>
		</Pane>
	</Splitpanes>
</div>

<style>
	:global(.splitpanes-remove-splitter > .splitpanes__pane) {
		background-color: inherit !important;
	}
	:global(.splitpanes-remove-splitter > .splitpanes__splitter) {
		background-color: transparent !important;
		width: 0 !important;
		border: none !important;
	}

	:global(.splitpanes__pane) {
		overflow-y: auto;
		scrollbar-width: none;
	}

	:global(.splitpanes__pane:hover) {
		scrollbar-width: thin;
	}
</style>
