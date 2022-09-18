<script context="module" lang="ts">
	export type ToggleButtonContext = {
		selected: Writable<string[]>
		select: (value: string) => void
	}
</script>

<script lang="ts">
	import { setContext } from 'svelte'
	import { writable, type Writable } from 'svelte/store'

	export let exclusive: boolean = true
	export let selected: string[] = []

	const selectedContent = writable(selected)

	setContext<ToggleButtonContext>('ToggleButtonGroup', {
		selected: selectedContent,
		select: (value: string) => {
			if (exclusive) {
				selectedContent.set([value])
			} else {
				selectedContent.update((selected) => {
					const index = selected.findIndex((val: string) => val === value)
					if (index !== -1) {
						selected.splice(index, 1)
						return selected
					} else {
						return [value, ...selected]
					}
				})
			}
		}
	})
</script>

<div class="inline-flex rounded-md shadow-sm" role="group">
	<slot />
</div>
