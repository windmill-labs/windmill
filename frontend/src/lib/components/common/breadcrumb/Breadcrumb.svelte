<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import Button from '../button/Button.svelte'

	export let items: string[]
	export let selectedIndex: number
	export let disabled: boolean = false

	const dispatch = createEventDispatcher()
</script>

<div class="flex items-center justify-center">
	{#each items as item, index}
		{#if index > 0}
			<slot name="separator" />
		{/if}
		<Button
			size="sm"
			color="light"
			btnClasses={selectedIndex - 1 === index ? 'text-gray-800 !font-bold' : '!text-primary'}
			on:click={() => dispatch('select', { index })}
			disabled={selectedIndex - 1 === index ? disabled : false}
		>
			{item}
		</Button>
	{/each}
</div>
