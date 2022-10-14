<script context="module" lang="ts">
	type InsertionMode = 'append' | 'connect' | 'insert'

	type SelectCallback = (path: string) => void

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
	import { createEventDispatcher, setContext } from 'svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { writable, type Writable } from 'svelte/store'

	export let pickableProperties: Object = {}
	export let displayContext = true

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
	<Pane minSize={20} class="relative p-4">
		<slot />
	</Pane>
	<Pane minSize={20} class="">
		<PropPicker
			{displayContext}
			{pickableProperties}
			on:select={({ detail }) => {
				dispatch('select', detail)
				$propPickerConfig?.onSelect(detail)
				propPickerConfig.set(undefined)
			}}
		/>
	</Pane>
</Splitpanes>
