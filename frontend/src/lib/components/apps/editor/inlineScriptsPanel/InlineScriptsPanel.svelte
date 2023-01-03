<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '../../types'
	import SplitPanesWrapper from '$lib/components/splitPanes/SplitPanesWrapper.svelte'
	import { Pane } from 'svelte-splitpanes'
	import InlineScriptsPanelList from './InlineScriptsPanelList.svelte'
	import InlineScriptEditorPanel from './InlineScriptEditorPanel.svelte'
	import InlineScriptEditor from './InlineScriptEditor.svelte'

	const { lazyGrid, app } = getContext<AppEditorContext>('AppEditorContext')

	let selectedScriptComponentId: string | undefined = undefined
</script>

<SplitPanesWrapper>
	<Pane size={25}>
		<InlineScriptsPanelList bind:selectedScriptComponentId />
	</Pane>
	<Pane size={75}>
		{#each $lazyGrid as gridComponent, index (index)}
			{#if gridComponent.data.id === selectedScriptComponentId}
				<InlineScriptEditorPanel bind:componentInput={gridComponent.data.componentInput} />
			{/if}

			{#if gridComponent.data.type === 'tablecomponent'}
				{#each gridComponent.data.actionButtons as actionButton, index (index)}
					{#if actionButton.id === selectedScriptComponentId}
						<InlineScriptEditorPanel bind:componentInput={actionButton.componentInput} />
					{/if}
				{/each}
			{/if}
		{/each}
		{#each $app.unusedInlineScripts as unusedInlineScript, index (index)}
			{#if `unused-${index}` === selectedScriptComponentId}
				<InlineScriptEditor
					bind:name={unusedInlineScript.name}
					bind:inlineScript={unusedInlineScript.inlineScript}
					on:delete={() => {
						// remove the script from the array at the index
						$app.unusedInlineScripts.splice(index, 1)
						$app.unusedInlineScripts = [...$app.unusedInlineScripts]
					}}
				/>
			{/if}
		{/each}
	</Pane>
</SplitPanesWrapper>
