<script lang="ts">
	import type { GridItem } from '$lib/components/apps/types'
	import Output from './Output.svelte'

	export let gridItem: GridItem
</script>

<div class="my-1">
	{#if gridItem.data.type === 'menucomponent' && gridItem.data.menuItems.length > 0}
		<div class="ml-2 border-l">
			{#each gridItem.data.menuItems as action, index}
				<Output id={action.id} first={index === 0} label="Menu item" />
			{/each}
		</div>
	{/if}
</div>
