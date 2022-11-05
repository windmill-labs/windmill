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
	import { createEventDispatcher, setContext } from 'svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { writable, type Writable } from 'svelte/store'

	export let pickableProperties: Object = {}
	export let displayContext = true
	export let priorId: string | undefined

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
	<Pane minSize={20} size={66} class="relative p-4 !transition-none">
		<slot />
	</Pane>
	<Pane minSize={20} size={34} class="px-2 py-2 h-full !transition-none">
		<PropPicker
			{priorId}
			{displayContext}
			{pickableProperties}
			on:select={({ detail }) => {
				dispatch('select', detail)
				if ($propPickerConfig?.onSelect(detail)) {
					propPickerConfig.set(undefined)
				}
			}}
		/>
	</Pane>
</Splitpanes>
