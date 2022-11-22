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
	const dispatch = createEventDispatcher()

	const selectedContent = writable(selected)

	$: $selectedContent && dispatch('select', $selectedContent)
	setContext<ToggleButtonContext>('ToggleButtonGroup', {
		selected: selectedContent,
		select: (value: any) => {
			selectedContent.set(value)
			selected = value
		}
	})
</script>

<div class="flex w-full rounded-md shadow-sm" role="group">
	<slot />
</div>
