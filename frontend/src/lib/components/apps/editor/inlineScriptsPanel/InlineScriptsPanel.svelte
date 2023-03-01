<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '../../types'
	import SplitPanesWrapper from '$lib/components/splitPanes/SplitPanesWrapper.svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import InlineScriptsPanelList from './InlineScriptsPanelList.svelte'
	import InlineScriptEditor from './InlineScriptEditor.svelte'
	import EmptyInlineScript from './EmptyInlineScript.svelte'
	import InlineScriptEditorPanel from './InlineScriptEditorPanel.svelte'

	const { app, staticOutputs, runnableComponents } =
		getContext<AppEditorContext>('AppEditorContext')

	let selectedScriptComponentId: string | undefined = undefined
</script>

<SplitPanesWrapper>
	<Splitpanes class="!overflow-visible">
		<Pane size={25}>
			<InlineScriptsPanelList bind:selectedScriptComponentId />
		</Pane>
		<Pane size={75}>
			{#if !selectedScriptComponentId}
				<div class="text-sm text-gray-500 text-center py-8 px-2">
					Select a script on the left panel
				</div>
			{/if}

			{#each $app.grid as gridItem (gridItem.data.id)}
				{#if gridItem.data.id === selectedScriptComponentId}
					<InlineScriptEditorPanel
						id={gridItem.data.id}
						bind:componentInput={gridItem.data.componentInput}
					/>
				{/if}

				{#if gridItem.data.type === 'tablecomponent'}
					{#each gridItem.data.actionButtons as actionButton, index (index)}
						{#if actionButton.id === selectedScriptComponentId}
							<InlineScriptEditorPanel
								id={actionButton.id}
								bind:componentInput={actionButton.componentInput}
							/>
						{/if}
					{/each}
				{/if}
			{/each}

			{#if $app.subgrids}
				{#each Object.keys($app.subgrids ?? {}) as key (key)}
					{#each $app.subgrids[key] as gridItem (gridItem.data.id)}
						{#if gridItem.data.id === selectedScriptComponentId}
							<InlineScriptEditorPanel
								id={gridItem.data.id}
								bind:componentInput={gridItem.data.componentInput}
							/>
						{/if}

						{#if gridItem.data.type === 'tablecomponent'}
							{#each gridItem.data.actionButtons as actionButton, index (index)}
								{#if actionButton.id === selectedScriptComponentId}
									<InlineScriptEditorPanel
										id={actionButton.id}
										bind:componentInput={actionButton.componentInput}
									/>
								{/if}
							{/each}
						{/if}
					{/each}
				{/each}
			{/if}

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
			{#each $app?.hiddenInlineScripts ?? [] as hiddenInlineScript, index (index)}
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
								delete $runnableComponents[`bg_${index}`]
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
