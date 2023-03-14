<script lang="ts">
	import type { GridItem } from '$lib/components/apps/types'
	import TableActionOutput from './TableActionOutput.svelte'

	export let gridItem: GridItem
</script>

{#if gridItem.data.type === 'tablecomponent'}
	<div class="py-1 ml-2 border">
		{#each gridItem.data.actionButtons as action}
			<TableActionOutput id={action.id} />
		{/each}
	</div>
{/if}
