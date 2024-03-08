<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { twMerge } from 'tailwind-merge'

	export let hoverable: boolean = false
	export let selected: boolean = false
	export let dividable: boolean = false
	const dispatch = createEventDispatcher()
</script>

<tr
	class={twMerge(
		hoverable ? 'hover:bg-surface-hover cursor-pointer' : '',
		selected ? 'bg-blue-50 dark:bg-blue-900/50' : '',
		'transition-all',
		dividable ? 'divide-x' : ''
	)}
	on:click={() => {
		dispatch('click')
	}}
>
	<slot />
</tr>
