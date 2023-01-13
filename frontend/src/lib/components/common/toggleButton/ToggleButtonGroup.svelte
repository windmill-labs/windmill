<script context="module" lang="ts">
	export type ToggleButtonContext = {
		selected: Writable<any>
		select: (value: any) => void
	}
</script>

<script lang="ts">
	import { createEventDispatcher, setContext } from 'svelte'
	import { writable, type Writable } from 'svelte/store'

	export let selected: any
	export let col = false

	const dispatch = createEventDispatcher()
	const selectedContent = writable(selected)

	function setSelected(selected: any) {
		selectedContent.set(selected)
	}

	$: setSelected(selected)

	$: $selectedContent && dispatch('selected', $selectedContent)
	setContext<ToggleButtonContext>('ToggleButtonGroup', {
		selected: selectedContent,
		select: (value: any) => {
			selectedContent.set(value)
			selected = value
		}
	})
</script>

<div class:flex-col={col} class="flex rounded-md {$$props.class}" role="group">
	<slot />
</div>
