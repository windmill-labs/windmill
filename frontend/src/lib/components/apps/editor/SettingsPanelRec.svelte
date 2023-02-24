<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext, GridItem } from '../types'

	import ComponentPanel from './settingsPanel/ComponentPanel.svelte'
	import SettingsPanelList from './SettingsPanelList.svelte'
	import TablePanel from './TablePanel.svelte'

	const { selectedComponent } = getContext<AppEditorContext>('AppEditorContext')

	export let gridItems: GridItem[]
</script>

{#if gridItems}
	{#each gridItems as gridItem (gridItem.data.id)}
		{#if gridItem.data.id === $selectedComponent}
			<ComponentPanel {gridItems} bind:component={gridItem.data} />
		{:else if gridItem.data.type === 'tablecomponent'}
			<TablePanel bind:component={gridItem.data} />
		{:else if gridItem.data.subGrids}
			<SettingsPanelList bind:subgrids={gridItem.data.subGrids} />
		{/if}
	{/each}
{/if}
