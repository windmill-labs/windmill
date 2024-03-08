<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext, GridItem } from '../../types'
	import InlineScriptEditorPanel from './InlineScriptEditorPanel.svelte'

	const { selectedComponentInEditor } = getContext<AppEditorContext>('AppEditorContext')

	export let gridItem: GridItem
</script>

{#if gridItem?.data?.id === $selectedComponentInEditor || gridItem?.data?.id + '_transformer' === $selectedComponentInEditor}
	<InlineScriptEditorPanel
		on:createScriptFromInlineScript
		defaultUserInput={gridItem.data?.type == 'formcomponent' ||
			gridItem?.data?.type == 'formbuttoncomponent'}
		id={gridItem.data.id}
		componentType={gridItem.data.type}
		transformer={$selectedComponentInEditor?.endsWith('_transformer')}
		bind:componentInput={gridItem.data.componentInput}
	/>
{/if}

{#if gridItem?.data?.type === 'tablecomponent'}
	{#each gridItem.data.actionButtons as actionButton, index (index)}
		{#if actionButton?.id === $selectedComponentInEditor || actionButton?.id + '_transformer' === $selectedComponentInEditor}
			<InlineScriptEditorPanel
				on:createScriptFromInlineScript
				componentType={actionButton.type}
				id={actionButton.id}
				transformer={$selectedComponentInEditor?.endsWith('_transformer')}
				bind:componentInput={actionButton.componentInput}
			/>
		{/if}
	{/each}
{/if}

{#if gridItem?.data?.type === 'menucomponent'}
	{#each gridItem.data.menuItems as actionButton, index (index)}
		{#if actionButton?.id === $selectedComponentInEditor || actionButton?.id + '_transformer' === $selectedComponentInEditor}
			<InlineScriptEditorPanel
				on:createScriptFromInlineScript
				componentType={actionButton.type}
				id={actionButton.id}
				transformer={$selectedComponentInEditor?.endsWith('_transformer')}
				bind:componentInput={actionButton.componentInput}
			/>
		{/if}
	{/each}
{/if}
