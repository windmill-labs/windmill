<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext, GridItem, TableComponent } from '../types'
	import ComponentPanel from './settingsPanel/ComponentPanel.svelte'
	import TablePanel from './TablePanel.svelte'

	const { selectedComponent, app } = getContext<AppEditorContext>('AppEditorContext')
	$: setTimeout(() => ($app.grid = $app.grid))
</script>

{#each $app.grid as gridItem, i (gridItem.data.id)}
	{#if gridItem.data.id === $selectedComponent}
		<ComponentPanel bind:component={gridItem.data} />
	{:else if gridItem.data.type === 'tablecomponent'}
		<TablePanel bind:component={gridItem.data} />
	{/if}
{/each}
