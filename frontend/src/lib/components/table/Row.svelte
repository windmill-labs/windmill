<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { twMerge } from 'tailwind-merge'

	export let hoverable: boolean = false
	export let selected: boolean = false
	export let dividable: boolean = false
	export let disabled: boolean = false
	export let hovering: boolean = false
	const dispatch = createEventDispatcher()
</script>

<tr
	class={twMerge(
		hoverable ? 'hover:bg-surface-hover cursor-pointer' : '',
		selected ? 'bg-blue-50 dark:bg-blue-900/50' : '',
		'transition-all',
		dividable ? 'divide-x' : '',
		disabled ? 'opacity-60' : '',
		$$props.class
	)}
	on:click={() => {
		dispatch('click')
	}}
	on:mouseenter={() => {
		hovering = true
		dispatch('hover', true)
	}}
	on:mouseleave={() => {
		hovering = false
		dispatch('hover', false)
	}}
>
	<slot />
</tr>
