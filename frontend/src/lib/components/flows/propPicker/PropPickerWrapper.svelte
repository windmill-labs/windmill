<script context="module" lang="ts">
	type InsertionMode = 'append' | 'connect' | 'insert'

	type SelectCallback = (path: string) => boolean

	type PropPickerConfig = {
		insertionMode: InsertionMode
		propName: string
		onSelect: SelectCallback
	}

	export type PropPickerWrapperContext = {
		propPickerConfig: Writable<PropPickerConfig | undefined>
		focusProp: (propName: string, insertionMode: InsertionMode, onSelect: SelectCallback) => void
		clearFocus: () => void
	}
</script>

<script lang="ts">
	import PropPicker from '$lib/components/propertyPicker/PropPicker.svelte'
	import PropPickerResult from '$lib/components/propertyPicker/PropPickerResult.svelte'
	import { clickOutside } from '$lib/utils'
	import { createEventDispatcher, setContext } from 'svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { writable, type Writable } from 'svelte/store'
	import type { PickableProperties } from '../previousResults'
	import { twMerge } from 'tailwind-merge'

	export let pickableProperties: PickableProperties | undefined
	export let result: any = undefined
	export let extraResults: any = undefined
	export let flow_input: any = undefined
	export let error: boolean = false
	export let displayContext = true
	export let notSelectable = false
	export let noPadding: boolean = false

	const propPickerConfig = writable<PropPickerConfig | undefined>(undefined)
	const dispatch = createEventDispatcher()

	setContext<PropPickerWrapperContext>('PropPickerWrapper', {
		propPickerConfig,
		focusProp: (propName, insertionMode, onSelect) => {
			propPickerConfig.set({
				propName,
				insertionMode,
				onSelect
			})
		},
		clearFocus: () => {
			propPickerConfig.set(undefined)
		}
	})
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
		<Pane
			minSize={20}
			size={40}
			class="pt-2 relative ml-[-1px] border-4 !transition-none z-1000 {$propPickerConfig
				? 'rounded-l-md border-blue-500'
				: 'border-transparent'}"
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
