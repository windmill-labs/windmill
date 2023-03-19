<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext, AppViewerContext } from '../../types'
	import SplitPanesWrapper from '$lib/components/splitPanes/SplitPanesWrapper.svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import InlineScriptsPanelList from './InlineScriptsPanelList.svelte'
	import InlineScriptEditor from './InlineScriptEditor.svelte'
	import EmptyInlineScript from './EmptyInlineScript.svelte'
	import InlineScriptsPanelWithTable from './InlineScriptsPanelWithTable.svelte'

	const { app, runnableComponents } = getContext<AppViewerContext>('AppViewerContext')
	const { selectedComponentInEditor } = getContext<AppEditorContext>('AppEditorContext')

	function deleteBackgroundScript(index: number) {
		// remove the script from the array at the index
		$app.hiddenInlineScripts.splice(index, 1)
		$app.hiddenInlineScripts = [...$app.hiddenInlineScripts]

		delete $runnableComponents[`bg_${index}`]
	}
</script>

<SplitPanesWrapper>
	<Splitpanes class="!overflow-visible">
		<Pane size={25}>
			<InlineScriptsPanelList />
		</Pane>
		<Pane size={75}>
			{#if !$selectedComponentInEditor}
				<div class="text-sm text-gray-500 text-center py-8 px-2">
					Select a script on the left panel
				</div>
			{/if}

			{#each $app.grid as gridItem (gridItem.id)}
				<InlineScriptsPanelWithTable bind:gridItem />
			{/each}

			{#each Object.keys($app.subgrids ?? {}) as key (key)}
				{#each $app?.subgrids?.[key] ?? [] as gridItem (gridItem.id)}
					<InlineScriptsPanelWithTable bind:gridItem />
				{/each}
			{/each}

			{#each $app.unusedInlineScripts as unusedInlineScript, index (index)}
				{#if `unused-${index}` === $selectedComponentInEditor}
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
				{#if `bg_${index}` === $selectedComponentInEditor}
					{#if hiddenInlineScript.inlineScript}
						<InlineScriptEditor
							id={`bg_${index}`}
							bind:inlineScript={hiddenInlineScript.inlineScript}
							bind:name={hiddenInlineScript.name}
							bind:fields={hiddenInlineScript.fields}
							syncFields
							on:delete={() => deleteBackgroundScript(index)}
						/>
					{:else}
						<EmptyInlineScript
							id={`b_${index}`}
							name={hiddenInlineScript.name}
							on:delete={() => deleteBackgroundScript(index)}
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
