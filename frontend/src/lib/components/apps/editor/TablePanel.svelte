<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '../types'
	import type { TableComponent } from './Component.svelte'
	import ComponentPanel from './settingsPanel/ComponentPanel.svelte'

	export let component: TableComponent
	const { selectedComponent } = getContext<AppEditorContext>('AppEditorContext')
</script>

{#each component.actionButtons as actionButton (actionButton.id)}
	{#if actionButton.id === $selectedComponent}
		<ComponentPanel
			rowColumns
			bind:component={actionButton}
			onDelete={() => {
				component.actionButtons = component.actionButtons.filter((c) => c.id !== actionButton.id)
			}}
		/>
	{/if}
{/each}
