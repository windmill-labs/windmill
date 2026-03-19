<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext, AppViewerContext, GridItem } from '../../types'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import InlineScriptsPanelList from './InlineScriptsPanelList.svelte'
	import InlineScriptEditor from './InlineScriptEditor.svelte'
	import InlineScriptsPanelWithTable from './InlineScriptsPanelWithTable.svelte'
	import InlineScriptHiddenRunnable from './InlineScriptHiddenRunnable.svelte'
	import { sendUserToast } from '$lib/toast'
	import { workspaceStore } from '$lib/stores'
	import { twMerge } from 'tailwind-merge'
	import { createScriptFromInlineScript } from './utils'
	import { BG_PREFIX } from '../appUtilsCore'

	const { app, runnableComponents, appPath } = getContext<AppViewerContext>('AppViewerContext')
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
				type: 'inline',
				recomputeIds: undefined
			}
			$app.hiddenInlineScripts = $app.hiddenInlineScripts
		}

		$selectedComponentInEditor = undefined
		if (runnableComponents) {
			delete $runnableComponents[BG_PREFIX + index]
			$runnableComponents = $runnableComponents
		}
	}

	let prefixOrId = $derived.by(() => {
		const sel = $selectedComponentInEditor
		if (!sel) return undefined
		if (sel.startsWith(BG_PREFIX)) return 'bg'
		return sel.endsWith('_transformer') ? sel.slice(0, -'_transformer'.length) : sel
	})
	let id = $derived.by(() => {
		const sel = $selectedComponentInEditor
		if (!sel || !sel.startsWith(BG_PREFIX)) return undefined
		const rest = sel.slice(BG_PREFIX.length)
		return rest.endsWith('_transformer') ? rest.slice(0, -'_transformer'.length) : rest
	})

	function containsAction(gridItem: GridItem, actionId: string | undefined): boolean {
		if (!actionId || !gridItem?.data) return false
		const data = gridItem.data
		if (data.type === 'tablecomponent') {
			return data.actionButtons?.some((a) => a.id === actionId) ?? false
		}
		if (
			data.type === 'aggridcomponent' ||
			data.type === 'aggridcomponentee' ||
			data.type === 'dbexplorercomponent' ||
			data.type === 'aggridinfinitecomponent' ||
			data.type === 'aggridinfinitecomponentee'
		) {
			return data.actions?.some((a) => a.id === actionId) ?? false
		}
		if (data.type === 'menucomponent') {
			return data.menuItems?.some((a) => a.id === actionId) ?? false
		}
		return false
	}

	interface Props {
		width?: number | undefined
	}

	let { width = undefined }: Props = $props()
</script>

<Splitpanes
	class={twMerge('!overflow-visible')}
	style={width !== undefined ? `width:${width}px;` : 'width: 100%;'}
>
	<Pane size={25}>
		<InlineScriptsPanelList on:hidePanel />
	</Pane>
	<Pane size={75}>
		{#if !$selectedComponentInEditor}
			<div class="text-sm text-secondary text-center py-8 px-2">
				Select a runnable on the left panel
			</div>
		{:else if prefixOrId != 'bg' && !prefixOrId?.startsWith('unused-')}
			{#each $app.grid as gridItem, index (gridItem?.id)}
				{#if gridItem?.id == prefixOrId || containsAction(gridItem, prefixOrId)}
					<InlineScriptsPanelWithTable
						on:createScriptFromInlineScript={(e) => {
							createScriptFromInlineScript(
								gridItem?.id ?? 'unknown',
								e.detail,
								$workspaceStore ?? '',
								$appPath
							)
						}}
						bind:gridItem={$app.grid[index]}
					/>
				{/if}
			{/each}
			{#each Object.keys($app.subgrids ?? {}) as subgrid (subgrid)}
				{#each $app.subgrids?.[subgrid] ?? [] as subgridItem, index (subgridItem?.id)}
					{#if (subgridItem?.id == prefixOrId || containsAction(subgridItem, prefixOrId)) && $app.subgrids?.[subgrid]}
						<InlineScriptsPanelWithTable
							on:createScriptFromInlineScript={(e) => {
								createScriptFromInlineScript(
									subgridItem?.id ?? 'unknown',
									e.detail,
									$workspaceStore ?? '',
									$appPath
								)
							}}
							bind:gridItem={$app.subgrids[subgrid][index]}
						/>
					{/if}
				{/each}
			{/each}
		{:else if prefixOrId != 'bg' && prefixOrId?.startsWith('unused-')}
			{#each $app.unusedInlineScripts as unusedInlineScript, index}
				{#if `unused-${index}` == prefixOrId}
					<InlineScriptEditor
						on:createScriptFromInlineScript={() =>
							sendUserToast('Cannot save to workspace unused scripts', true)}
						id={`unused-${index}`}
						bind:name={unusedInlineScript.name}
						bind:inlineScript={unusedInlineScript.inlineScript}
						on:delete={() => {
							$app.unusedInlineScripts.splice(index, 1)
							$app.unusedInlineScripts = [...$app.unusedInlineScripts]
						}}
					/>
				{/if}
			{/each}
		{:else if prefixOrId == 'bg'}
			{#each $app.hiddenInlineScripts as _inlineScript, index}
				{#if index.toString() == id}
					<InlineScriptHiddenRunnable
						on:createScriptFromInlineScript={(e) => {
							createScriptFromInlineScript(
								BG_PREFIX + index,
								e.detail,
								$workspaceStore ?? '',
								$appPath
							)
							$app = $app
						}}
						transformer={$selectedComponentInEditor?.endsWith('_transformer')}
						on:delete={() => deleteBackgroundScript(index)}
						id={BG_PREFIX + index}
						bind:runnable={$app.hiddenInlineScripts[index]}
					/>
				{/if}
			{/each}
		{:else}
			<div class="text-sm text-primary text-center py-8 px-2">
				No runnable found at id {$selectedComponentInEditor}
			</div>
		{/if}
	</Pane>
</Splitpanes>
