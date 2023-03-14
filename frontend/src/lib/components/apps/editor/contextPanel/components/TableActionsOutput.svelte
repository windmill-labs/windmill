<script lang="ts">
	import type { GridItem } from '$lib/components/apps/types'
	import TableActionOutput from './TableActionOutput.svelte'

	export let gridItem: GridItem
	export let expanded: boolean = false
</script>

{#if gridItem.data.type === 'tablecomponent' && gridItem.data.actionButtons.length > 0}
	<div class="py-1 ml-2 border">
		{#each gridItem.data.actionButtons as action}
			<TableActionOutput id={action.id} {expanded} />
		{/each}
	</div>
{/if}
