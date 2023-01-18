<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '../../types'
	import SplitPanesWrapper from '$lib/components/splitPanes/SplitPanesWrapper.svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import InlineScriptsPanelList from './InlineScriptsPanelList.svelte'
	import InlineScriptEditorPanel from './InlineScriptEditorPanel.svelte'
	import InlineScriptEditor from './InlineScriptEditor.svelte'
	import EmptyInlineScript from './EmptyInlineScript.svelte'

	const { lazyGrid, app, staticOutputs } = getContext<AppEditorContext>('AppEditorContext')

	let selectedScriptComponentId: string | undefined = undefined
</script>

<SplitPanesWrapper>
	<Splitpanes class="!overflow-visible">
		<Pane size={25}>
			<InlineScriptsPanelList bind:selectedScriptComponentId />
		</Pane>
		<Pane size={75}>
			{#each $lazyGrid as gridComponent, index (index)}
				{#if gridComponent.data.id === selectedScriptComponentId}
					<InlineScriptEditorPanel
						id={gridComponent.data.id}
						bind:componentInput={gridComponent.data.componentInput}
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
			{/each}
			{#each $app.unusedInlineScripts as unusedInlineScript, index (index)}
				{#if `unused-${index}` === selectedScriptComponentId}
					<InlineScriptEditor
						id={`unused-${index}`}
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
			{#each $app.hiddenInlineScripts as hiddenInlineScript, index (index)}
				{#if `bg_${index}` === selectedScriptComponentId}
					{#if hiddenInlineScript.inlineScript}
						<InlineScriptEditor
							id={`bg_${index}`}
							bind:inlineScript={hiddenInlineScript.inlineScript}
							bind:name={hiddenInlineScript.name}
							on:delete={() => {
								// remove the script from the array at the index
								$app.hiddenInlineScripts.splice(index, 1)
								$app.hiddenInlineScripts = [...$app.hiddenInlineScripts]

								delete $staticOutputs[`bg_${index}`]
								$staticOutputs = $staticOutputs
							}}
						/>
					{:else}
						<EmptyInlineScript
							id={`b_${index}`}
							name={hiddenInlineScript.name}
							on:new={(e) => {
								hiddenInlineScript.inlineScript = e.detail
							}}
						/>
					{/if}
				{/if}
			{/each}
		</Pane>
	</Splitpanes>
</SplitPanesWrapper>
