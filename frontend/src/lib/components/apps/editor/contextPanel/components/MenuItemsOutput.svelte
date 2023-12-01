<script lang="ts">
	import type { GridItem } from '$lib/components/apps/types'
	import MenuItemOutput from './MenuItemOutput.svelte'

	export let gridItem: GridItem
</script>

<div class="my-1">
	{#if gridItem.data.type === 'menucomponent' && gridItem.data.menuItems.length > 0}
		<div class="ml-2 border-l">
			{#each gridItem.data.menuItems as action, index}
				<MenuItemOutput id={action.id} first={index === 0} />
			{/each}
		</div>
	{/if}
</div>
