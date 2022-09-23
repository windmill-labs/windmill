<script context="module" lang="ts">
	export type TabsContext = {
		selected: Writable<string>
		update: (value: string) => void
	}
</script>

<script lang="ts">
	import { setContext } from 'svelte'
	import { writable, type Writable } from 'svelte/store'

	export let selected: string

	$: selected && updateSelected()

	const selectedContent = writable(selected)

	setContext<TabsContext>('Tabs', {
		selected: selectedContent,
		update: (value: string) => {
			selectedContent.set(value)
			selected = value
		}
	})

	function updateSelected() {
		selectedContent.set(selected)
	}
</script>

<div class="border-b border-gray-200 flex flex-row">
	<slot />
</div>
<slot name="content" />
