<script lang="ts">
	import type { GridItem } from '$lib/components/apps/types'
	import Output from './Output.svelte'

	interface Props {
		gridItem: GridItem
		render: boolean
	}

	let { gridItem, render }: Props = $props()
</script>

<div class="my-1">
	{#if gridItem.data.type === 'tablecomponent' && gridItem.data.actionButtons.length > 0}
		<div class="ml-2 border-l">
			{#each gridItem.data.actionButtons as action, index}
				<Output
					renderRec={render}
					id={action.id}
					first={index === 0}
					label="Table action"
					numberOfSubgrids={action.numberOfSubgrids ?? 0}
				/>
			{/each}
		</div>
	{/if}
	{#if (gridItem.data.type === 'aggridcomponent' || gridItem.data.type === 'aggridcomponentee' || gridItem.data.type === 'dbexplorercomponent' || gridItem.data.type === 'aggridinfinitecomponent' || gridItem.data.type === 'aggridinfinitecomponentee') && gridItem.data.actions?.length > 0}
		<div class="ml-2 border-l">
			{#each gridItem.data.actions as action, index}
				<Output
					renderRec={render}
					id={action.id}
					first={index === 0}
					label="Table action"
					numberOfSubgrids={action.numberOfSubgrids ?? 0}
				/>
			{/each}
		</div>
	{/if}
</div>
