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
	import { createEventDispatcher, setContext } from 'svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { writable, type Writable } from 'svelte/store'
	import type { PickableProperties } from '../previousResults'

	export let pickableProperties: PickableProperties | undefined
	export let result: any = undefined
	export let error: boolean = false
	export let displayContext = true
	export let notSelectable = false

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

<Splitpanes>
	<Pane minSize={20} size={60} class="relative p-4 !transition-none">
		<slot />
	</Pane>
	<Pane minSize={20} size={40} class="py-2 relative !transition-none">
		{#if result}
			<PropPickerResult {result} />
		{:else if pickableProperties}
			<PropPicker
				{displayContext}
				{error}
				{pickableProperties}
				{notSelectable}
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
