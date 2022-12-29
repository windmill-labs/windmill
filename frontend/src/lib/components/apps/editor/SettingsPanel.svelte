<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '../types'
	import ComponentPanel from './settingsPanel/ComponentPanel.svelte'
	import TablePanel from './TablePanel.svelte'

	const { selectedComponent, lazyGrid } = getContext<AppEditorContext>('AppEditorContext')
</script>

{#each $lazyGrid as gridItem (gridItem.data.id)}
	{#if gridItem.data.id === $selectedComponent}
		<ComponentPanel bind:component={gridItem.data} />
	{:else if gridItem.data.type === 'tablecomponent'}
		<TablePanel bind:component={gridItem.data} />
	{/if}
{/each}
