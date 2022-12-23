<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext, TableComponent } from '../types'
	import ComponentPanel from './settingsPanel/ComponentPanel.svelte'

	export let component: TableComponent
	const { selectedComponent } = getContext<AppEditorContext>('AppEditorContext')
</script>

<!-- {JSON.stringify(component)} -->

{#each component.actionButtons as actionButton (actionButton.id)}
	{#if actionButton.id === $selectedComponent}
		{JSON.stringify(actionButton.componentInput.fields)}
		<ComponentPanel
			bind:component={actionButton}
			onDelete={() => {
				component.actionButtons = component.actionButtons.filter((c) => c.id !== actionButton.id)
			}}
		/>
	{/if}
{/each}
