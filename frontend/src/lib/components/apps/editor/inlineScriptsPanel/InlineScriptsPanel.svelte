<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext, AppViewerContext, HiddenRunnable } from '../../types'
	import SplitPanesWrapper from '$lib/components/splitPanes/SplitPanesWrapper.svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import InlineScriptsPanelList from './InlineScriptsPanelList.svelte'
	import InlineScriptEditor from './InlineScriptEditor.svelte'
	import InlineScriptsPanelWithTable from './InlineScriptsPanelWithTable.svelte'
	import { findGridItem } from '../appUtils'
	import InlineScriptHiddenRunnable from './InlineScriptHiddenRunnable.svelte'
	import { BG_PREFIX } from '../../utils'
	import { sendUserToast } from '$lib/toast'
	import type { RunnableByName } from '../../inputType'
	import { ScriptService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { findNextAvailablePath } from '$lib/path'

	const { app, runnableComponents } = getContext<AppViewerContext>('AppViewerContext')
	const { selectedComponentInEditor } = getContext<AppEditorContext>('AppEditorContext')

	function deleteBackgroundScript(index: number) {
		// remove the script from the array at the index
		if ($app.hiddenInlineScripts.length - 1 == index) {
			$app.hiddenInlineScripts.splice(index, 1)
			$app.hiddenInlineScripts = [...$app.hiddenInlineScripts]
		} else {
			$app.hiddenInlineScripts[index] = {
				hidden: true,
				inlineScript: undefined,
				name: `Background Runnable ${index}`,
				fields: {},
				type: 'runnableByName',
				recomputeIds: undefined
			}
			$app.hiddenInlineScripts = $app.hiddenInlineScripts
		}

		$selectedComponentInEditor = undefined
		delete $runnableComponents[BG_PREFIX + index]
		$runnableComponents = $runnableComponents
	}

	$: gridItem =
		$selectedComponentInEditor && !$selectedComponentInEditor.startsWith(BG_PREFIX)
			? findGridItem($app, $selectedComponentInEditor?.split('_')?.[0])
			: undefined

	$: hiddenInlineScript = $app?.hiddenInlineScripts?.findIndex((k_, index) => {
		const [prefix, id] = $selectedComponentInEditor?.split('_') || []

		if (prefix !== 'bg') return false

		return Number(id) === index
	})

	$: unusedInlineScript = $app?.unusedInlineScripts?.findIndex(
		(k_, index) => `unused-${index}` === $selectedComponentInEditor
	)

	async function createScriptFromInlineScript(
		id: string,
		runnable: HiddenRunnable | RunnableByName
	) {
		if (runnable.type != 'runnableByName') {
			sendUserToast('Only inline scripts can be saved to workspace', true)
			return
		}
		if (!runnable.inlineScript) {
			sendUserToast('No inline script found', true)
			return
		}
		let path = `${runnable.inlineScript.path}/inline_${id}`
		path = await findNextAvailablePath(path)
		let language = runnable.inlineScript.language
		if (language == 'frontend') {
			sendUserToast('Frontend scripts can not be saved to workspace', true)
			return
		}
		await ScriptService.createScript({
			workspace: $workspaceStore!,
			requestBody: {
				path: path,
				summary: runnable.name ?? '',
				description: '',
				content: runnable.inlineScript.content,
				parent_hash: undefined,
				schema: runnable.inlineScript.schema,
				is_template: false,
				language
			}
		})

		Object.assign(runnable, {
			type: 'runnableByPath',
			schema: runnable.inlineScript.schema,
			runType: 'script',
			recomputeIds: undefined,
			path
		})

		$app = $app
	}
</script>

<SplitPanesWrapper>
	<Splitpanes class="!overflow-visible">
		<Pane size={25}>
			<InlineScriptsPanelList />
		</Pane>
		<Pane size={75}>
			{#if !$selectedComponentInEditor}
				<div class="text-sm text-secondary text-center py-8 px-2">
					Select a script on the left panel
				</div>
			{:else if gridItem}
				{#key gridItem?.id}
					<InlineScriptsPanelWithTable
						on:createScriptFromInlineScript={(e) => {
							createScriptFromInlineScript(gridItem?.id ?? 'unknown', e.detail)
						}}
						bind:gridItem
					/>
				{/key}
			{:else if unusedInlineScript > -1 && $app.unusedInlineScripts?.[unusedInlineScript]}
				{#key unusedInlineScript}
					<InlineScriptEditor
						on:createScriptFromInlineScript={() =>
							sendUserToast('Cannot save to workspace unused scripts', true)}
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
					{#if $app.hiddenInlineScripts?.[hiddenInlineScript]}
						<InlineScriptHiddenRunnable
							on:createScriptFromInlineScript={(e) => {
								createScriptFromInlineScript(BG_PREFIX + hiddenInlineScript, e.detail)
							}}
							transformer={$selectedComponentInEditor?.endsWith('_transformer')}
							on:delete={() => deleteBackgroundScript(hiddenInlineScript)}
							id={BG_PREFIX + hiddenInlineScript}
							bind:runnable={$app.hiddenInlineScripts[hiddenInlineScript]}
						/>{/if}{/key}
			{:else}
				<div class="text-sm text-tertiary text-center py-8 px-2">
					No script found at id {$selectedComponentInEditor}
				</div>
			{/if}
		</Pane>
	</Splitpanes>
</SplitPanesWrapper>
