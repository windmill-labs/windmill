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
	import { setContext } from 'svelte'
	import { HSplitPane } from 'svelte-split-pane'
	import { writable, type Writable } from 'svelte/store'

	export let pickableProperties: Object = {}

	const propPickerConfig = writable<PropPickerConfig | undefined>(undefined)

	$: if (pickableProperties) {
		console.log('pickableProperties', pickableProperties)
	}

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

<div class="h-full overflow-hidden">
	<HSplitPane leftPaneSize="50%" rightPaneSize="50%" minLeftPaneSize="20%" minRightPaneSize="20%">
		<left slot="left" class="h-full">
			<div class="p-4 h-full overflow-y-auto">
				<slot />
			</div>
		</left>
		<right slot="right" class="h-full">
			<div class="p-4 h-full overflow-y-auto">
				<PropPicker
					{pickableProperties}
					on:select={({ detail }) => {
						$propPickerConfig?.onSelect(detail)
						propPickerConfig.set(undefined)
					}}
				/>
			</div>
		</right>
	</HSplitPane>
</div>
