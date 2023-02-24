<script lang="ts">
	import type { GridItem } from '../../types'
	import InlineScriptEditorList from './InlineScriptEditorList.svelte'

	import InlineScriptEditorPanel from './InlineScriptEditorPanel.svelte'

	export let gridItems: GridItem[]

	export let selectedScriptComponentId: string | undefined = undefined
</script>

{#if gridItems}
	{#each gridItems as gridComponent, index (index)}
		{#if gridComponent.data}
			{#if gridComponent.data.id === selectedScriptComponentId}
				<InlineScriptEditorPanel
					id={gridComponent.data.id}
					bind:componentInput={gridComponent.data.componentInput}
				/>
			{:else if gridComponent.data.subGrids}
				<InlineScriptEditorList
					{selectedScriptComponentId}
					bind:subgrids={gridComponent.data.subGrids}
				/>
			{/if}

			{#if gridComponent.data.type === 'tablecomponent'}
				{#each gridComponent.data.actionButtons as actionButton, index (index)}
					{#if actionButton.id === selectedScriptComponentId}
						<InlineScriptEditorPanel
							id={actionButton.id}
							bind:componentInput={actionButton.componentInput}
						/>
					{/if}
				{/each}
			{/if}
		{/if}
	{/each}
{/if}
