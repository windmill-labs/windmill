<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext, AppViewerContext } from '../../types'
	import SplitPanesWrapper from '$lib/components/splitPanes/SplitPanesWrapper.svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import InlineScriptsPanelList from './InlineScriptsPanelList.svelte'
	import InlineScriptEditor from './InlineScriptEditor.svelte'
	import EmptyInlineScript from './EmptyInlineScript.svelte'
	import InlineScriptsPanelWithTable from './InlineScriptsPanelWithTable.svelte'
	import { findGridItem } from '../appUtils'

	const { app, runnableComponents } = getContext<AppViewerContext>('AppViewerContext')
	const { selectedComponentInEditor } = getContext<AppEditorContext>('AppEditorContext')

	function deleteBackgroundScript(index: number) {
		// remove the script from the array at the index
		$app.hiddenInlineScripts.splice(index, 1)
		$app.hiddenInlineScripts = [...$app.hiddenInlineScripts]

		delete $runnableComponents[`bg_${index}`]
	}

	$: gridItem = $selectedComponentInEditor
		? findGridItem($app, $selectedComponentInEditor?.split('_transformer')?.[0])
		: undefined

	$: hiddenInlineScript = $app?.hiddenInlineScripts?.findIndex(
		(k_, index) => `bg_${index}` === $selectedComponentInEditor
	)

	$: unusedInlineScript = $app?.unusedInlineScripts?.findIndex(
		(k_, index) => `unused-${index}` === $selectedComponentInEditor
	)
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
			{:else if gridItem}
				{#key gridItem?.id}
					<InlineScriptsPanelWithTable bind:gridItem />
				{/key}
			{:else if unusedInlineScript > -1 && $app.unusedInlineScripts?.[unusedInlineScript]}
				{#key unusedInlineScript}
					<InlineScriptEditor
						id={`unused-${unusedInlineScript}`}
						bind:name={$app.unusedInlineScripts[unusedInlineScript].name}
						bind:inlineScript={$app.unusedInlineScripts[unusedInlineScript].inlineScript}
						on:delete={() => {
							// remove the script from the array at the index
							$app.unusedInlineScripts.splice(unusedInlineScript, 1)
							$app.unusedInlineScripts = [...$app.unusedInlineScripts]
						}}
					/>
				{/key}
			{:else if hiddenInlineScript > -1}
				{#key hiddenInlineScript}
					{#if $app.hiddenInlineScripts?.[hiddenInlineScript]?.inlineScript}
						<InlineScriptEditor
							id={`bg_${hiddenInlineScript}`}
							bind:inlineScript={$app.hiddenInlineScripts[hiddenInlineScript].inlineScript}
							bind:name={$app.hiddenInlineScripts[hiddenInlineScript].name}
							bind:fields={$app.hiddenInlineScripts[hiddenInlineScript].fields}
							syncFields
							on:delete={() => deleteBackgroundScript(hiddenInlineScript)}
						/>
					{:else}
						<EmptyInlineScript
							id={`b_${hiddenInlineScript}`}
							name={$app.hiddenInlineScripts[hiddenInlineScript].name}
							on:delete={() => deleteBackgroundScript(hiddenInlineScript)}
							on:new={(e) => {
								if ($app.hiddenInlineScripts[hiddenInlineScript]) {
									$app.hiddenInlineScripts[hiddenInlineScript].inlineScript = e.detail
								}
							}}
						/>
					{/if}
				{/key}
			{:else}
				<div class="text-sm text-gray-500 text-center py-8 px-2">
					No script found at id {$selectedComponentInEditor}
				</div>
			{/if}
		</Pane>
	</Splitpanes>
</SplitPanesWrapper>
