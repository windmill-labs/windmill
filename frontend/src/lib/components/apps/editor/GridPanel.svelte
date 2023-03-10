<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppViewerContext, GridItem } from '../types'
	import ComponentPanel from './settingsPanel/ComponentPanel.svelte'

	const { selectedComponent } = getContext<AppViewerContext>('AppViewerContext')
	export let gridItems: GridItem[]
	export let parent: string | undefined
</script>

{#each gridItems as gridItem, i (gridItem?.data?.id ?? i)}
	{#if gridItem?.data?.id === $selectedComponent}
		<ComponentPanel {parent} bind:component={gridItem.data} />
	{:else if gridItem?.data?.type === 'tablecomponent'}
		{#each gridItem.data.actionButtons as actionButton (actionButton.id)}
			{#if actionButton.id === $selectedComponent}
				<ComponentPanel
					parent={undefined}
					noGrid
					rowColumns
					bind:component={actionButton}
					duplicateMoveAllowed={false}
					onDelete={() => {
						gridItem.data.actionButtons = gridItem.data.actionButtons.filter(
							(c) => c.id !== actionButton.id
						)
					}}
				/>
			{/if}
		{/each}
	{/if}
{/each}
