<script lang="ts">
	import type { Snippet } from 'svelte'
	import Button from '../button/Button.svelte'

	let {
		items,
		selectedIndex,
		numbered = false,
		separator,
		onselect
	}: {
		items: string[]
		selectedIndex: number
		numbered?: boolean
		separator?: Snippet
		onselect?: (index: number) => void
	} = $props()
</script>

<div class="flex items-center justify-center">
	{#each items as item, index}
		{#if index > 0 && separator}
			{@render separator()}
		{/if}
		<Button
			unifiedSize="sm"
			variant="subtle"
			selected={selectedIndex - 1 === index}
			onClick={() => onselect?.(index)}
			disabled={index > selectedIndex - 1}
		>
			{numbered ? `${index + 1}. ` : ''}{item}
		</Button>
	{/each}
</div>
