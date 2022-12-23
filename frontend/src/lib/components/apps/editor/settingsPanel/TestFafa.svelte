<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '../../types'

	import ComponentPanel from './ComponentPanel.svelte'
	import TablePanel from '../TablePanel.svelte'

	export let rerender: string
	const { app, selectedComponent } = getContext<AppEditorContext>('AppEditorContext')

	$: rerender && ($app = $app)
</script>

{#each $app.grid as gridComponent, index (gridComponent.data.id + index)}
	{#if gridComponent.data.id === $selectedComponent}
		<ComponentPanel bind:component={gridComponent.data} />
	{:else if gridComponent.data.type === 'tablecomponent'}
		<TablePanel bind:component={gridComponent.data} />
	{/if}
{/each}
