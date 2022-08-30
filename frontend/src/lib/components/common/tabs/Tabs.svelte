<script lang="ts">
	import { setContext } from 'svelte'
	import { writable } from 'svelte/store'

	export let selected: string

	const selectedContent = writable(selected)

	setContext('Tabs', {
		selected: selectedContent,
		update: (value: string) => {
			selectedContent.set(value)
			selected = value
		}
	})
</script>

<div class="border-b border-gray-200 flex flex-row mb-4">
	<slot />
</div>
<slot name="content" />
