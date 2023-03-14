<script lang="ts">
	import type { GridItem } from '$lib/components/apps/types'
	import TableActionOutput from './TableActionOutput.svelte'

	export let gridItem: GridItem
	export let expanded: boolean = false
</script>

<div class="my-1">
	{#if gridItem.data.type === 'tablecomponent' && gridItem.data.actionButtons.length > 0}
		<div class="ml-2 border-l">
			{#each gridItem.data.actionButtons as action, index}
				<TableActionOutput id={action.id} {expanded} first={index === 0} />
			{/each}
		</div>
	{/if}
</div>
