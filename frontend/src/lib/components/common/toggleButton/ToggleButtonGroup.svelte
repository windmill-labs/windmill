<script context="module" lang="ts">
	export type ToggleButtonContext = {
		selected: Writable<string>
		select: (value: string) => void
	}
</script>

<script lang="ts">
	import { setContext } from 'svelte'
	import { writable, type Writable } from 'svelte/store'

	export let selected: string

	const selectedContent = writable(selected)

	setContext<ToggleButtonContext>('ToggleButtonGroup', {
		selected: selectedContent,
		select: (value: string) => {
			selectedContent.set(value)
			selected = value
		}
	})
</script>

<div class="inline-flex rounded-md shadow-sm" role="group">
	<slot />
</div>
