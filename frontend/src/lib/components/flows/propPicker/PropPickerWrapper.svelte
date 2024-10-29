<script lang="ts">
	import PropPicker from '$lib/components/propertyPicker/PropPicker.svelte'
	import PropPickerResult from '$lib/components/propertyPicker/PropPickerResult.svelte'
	import { clickOutside } from '$lib/utils'
	import { getContext, createEventDispatcher } from 'svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import type { PickableProperties } from '../previousResults'
	import { twMerge } from 'tailwind-merge'
	import AnimatedButton from '$lib/components/common/button/AnimatedButton.svelte'
	import type { FlowEditorContext } from '../types'
	import { writable } from 'svelte/store'
	import type { PropPickerConfig, InsertionMode, SelectCallback } from '../types'

	export let pickableProperties: PickableProperties | undefined
	export let result: any = undefined
	export let extraResults: any = undefined
	export let flow_input: any = undefined
	export let error: boolean = false
	export let displayContext = true
	export let notSelectable = false
	export let noPadding: boolean = false
	export let id: string

	const propPickerConfig = writable<PropPickerConfig | undefined>(undefined)
	const { flowInputsStore } = getContext<FlowEditorContext>('FlowEditorContext')
	const dispatch = createEventDispatcher()

	function setPropPickerConfig() {
		if ($flowInputsStore && id && $flowInputsStore?.[id] === undefined) {
			$flowInputsStore[id] = {
				connectingInputs: {
					propPickerConfig,
					focusProp: (propName: string, insertionMode: InsertionMode, onSelect: SelectCallback) => {
						propPickerConfig.set({
							propName,
							insertionMode,
							onSelect
						})
					},
					clearFocus: () => {
						propPickerConfig.set(undefined)
					}
				}
			}
		} else if ($flowInputsStore && id) {
			$flowInputsStore[id].connectingInputs = {
				propPickerConfig,
				focusProp: (propName: string, insertionMode: InsertionMode, onSelect: SelectCallback) => {
					propPickerConfig.set({
						propName,
						insertionMode,
						onSelect
					})
				},
				clearFocus: () => {
					propPickerConfig.set(undefined)
				}
			}
		}
	}

	$: $flowInputsStore && setPropPickerConfig()
</script>

<div
	class="h-full w-full"
	use:clickOutside
	on:click_outside={() => propPickerConfig.set(undefined)}
>
	<Splitpanes class="splitpanes-remove-splitter">
		<Pane class={twMerge('relative !transition-none ', noPadding ? '' : 'p-2')}>
			<slot />
		</Pane>
		<Pane minSize={20} size={40} class="!transition-none z-1000 ml-[-1px]">
			<AnimatedButton
				animate={$propPickerConfig?.insertionMode == 'connect'}
				baseRadius="4px"
				wrapperClasses="h-full w-full pt-2"
				marginWidth="4px"
				ringColor={$propPickerConfig?.insertionMode == 'insert' ||
				$propPickerConfig?.insertionMode == 'append'
					? '#3b82f6'
					: 'transparent'}
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
							//dispatch('select', detail)
							if ($propPickerConfig?.onSelect(detail)) {
								propPickerConfig.set(undefined)
							}
						}}
						{id}
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
