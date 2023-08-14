<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { twMerge } from 'tailwind-merge'

	export let hoverable: boolean = false
	export let selected: boolean = false
	const dispatch = createEventDispatcher()
</script>

<tr
	class={twMerge(
		hoverable ? 'hover:bg-surface-hover cursor-pointer' : '',
		selected ? 'bg-blue-50 dark:bg-blue-900/50' : '',
		'transition-all'
	)}
	on:click={() => {
		dispatch('click')
	}}
>
	<slot />
</tr>
