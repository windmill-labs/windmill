<script context="module" lang="ts">
	export type TabsContext = {
		selected: Writable<string>
		update: (value: string) => void
	}
</script>

<script lang="ts">
	import { setContext } from 'svelte'
	import { writable, type Writable } from 'svelte/store'
	import { createEventDispatcher } from 'svelte'

	const dispatch = createEventDispatcher()

	export let selected: string

	$: selected && updateSelected()

	const selectedContent = writable(selected)

	$: $selectedContent && dispatch('selected', $selectedContent)

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

<div
	class="border-b border-gray-200 flex flex-row whitespace-nowrap overflow-y-auto scrollbar-hidden {$$props.class}"
>
	<slot />
</div>
<slot name="content" />
