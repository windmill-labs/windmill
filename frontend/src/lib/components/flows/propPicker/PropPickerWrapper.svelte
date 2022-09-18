<script context="module" lang="ts">
	export type PropPickerWrapperContext = {
		focused: Writable<string | undefined>
		focus: (value: string | undefined) => void
	}
</script>

<script lang="ts">
	import PropPicker from '$lib/components/propertyPicker/PropPicker.svelte'
	import { setContext } from 'svelte'
	import { HSplitPane } from 'svelte-split-pane'
	import { writable, type Writable } from 'svelte/store'

	export let pickableProperties: Object = {}
	const focused = writable<string | undefined>(undefined)

	setContext<PropPickerWrapperContext>('PropPickerWrapper', {
		focused,
		focus: (value: string | undefined) => {
			focused.set(value)
		}
	})
</script>

<div class="h-full overflow-hidden">
	<HSplitPane leftPaneSize="50%" rightPaneSize="50%" minLeftPaneSize="20%" minRightPaneSize="20%">
		<left slot="left" class="h-full mr-4">
			<div class="p-4">
				<slot />
			</div>
		</left>
		<right slot="right" class="h-full">
			<div class="p-4">
				<PropPicker {pickableProperties} />
			</div>
		</right>
	</HSplitPane>
</div>
