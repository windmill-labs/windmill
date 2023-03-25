<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { getContext } from 'svelte'
	import type { App, AppViewerContext } from '../types'
	import { allItems } from '../utils'
	import { findGridItem } from './appUtils'
	import PanelSection from './settingsPanel/common/PanelSection.svelte'
	import ComponentPanel from './settingsPanel/ComponentPanel.svelte'
	import InputsSpecsEditor from './settingsPanel/InputsSpecsEditor.svelte'
	import BackgroundScriptTriggerList from './settingsPanel/triggerLists/BackgroundScriptTriggerList.svelte'

	const { selectedComponent, app, stateId } = getContext<AppViewerContext>('AppViewerContext')

	$: hiddenInlineScript = $app?.hiddenInlineScripts
		?.map((x, i) => ({ script: x, index: i }))
		.find(({ script, index }) => $selectedComponent?.includes(`bg_${index}`))

	$: componentSettings = findComponentSettings($app, $selectedComponent?.[0])

	$: tableActionSettings = findTableActionSettings($app, $selectedComponent?.[0])

	function findTableActionSettings(app: App, id: string | undefined) {
		return allItems(app.grid, app.subgrids)
			.map((x) => {
				if (x?.data?.type === 'tablecomponent') {
					if (x?.data?.actionButtons) {
						const tableAction = x.data.actionButtons.find((x) => x.id === id)
						if (tableAction) {
							return { item: { data: tableAction, id: tableAction.id }, parent: x.data.id }
						}
					}
				}
			})
			.find((x) => x)
	}

	function findComponentSettings(app: App, id: string | undefined) {
		if (!id) return
		if (app?.grid) {
			const gridItem = app.grid.find((x) => x.data?.id === id)
			if (gridItem) {
				return { item: gridItem, parent: undefined }
			}
		}

		if (app?.subgrids) {
			for (const key of Object.keys(app.subgrids ?? {})) {
				const gridItem = app.subgrids[key].find((x) => x.data?.id === id)
				if (gridItem) {
					return { item: gridItem, parent: key }
				}
			}
		}

		return undefined
	}
</script>

{#if componentSettings}
	{#key componentSettings?.item?.id}
		<ComponentPanel bind:componentSettings />
	{/key}
{:else if tableActionSettings}
	{#key tableActionSettings?.item?.data?.id}
		<ComponentPanel
			noGrid
			rowColumns
			bind:componentSettings={tableActionSettings}
			duplicateMoveAllowed={false}
			onDelete={() => {
				if (tableActionSettings) {
					const parent = findGridItem($app, tableActionSettings.parent)
					if (!parent) return

					if (parent.data.type === 'tablecomponent') {
						parent.data.actionButtons = parent.data.actionButtons.filter(
							(x) => x.id !== tableActionSettings?.item.id
						)
					}
				}
			}}
		/>
	{/key}
{:else if hiddenInlineScript}
	<div class="min-h-full flex flex-col divide-y">
		<PanelSection title={`Configuration`}>
			<div class="flex items-center">
				<Toggle
					bind:checked={hiddenInlineScript.script.autoRefresh}
					options={{ right: 'Run on start and app refresh' }}
				/>
				<Tooltip>
					You may want to disable this so that the background script is only triggered by changes to
					other values or triggered by another computation on a button (See 'Recompute Others')
				</Tooltip>
			</div>
		</PanelSection>

		<div class="p-2">
			{#if hiddenInlineScript.script.inlineScript}
				<BackgroundScriptTriggerList
					fields={hiddenInlineScript.script.fields}
					autoRefresh={hiddenInlineScript.script.autoRefresh}
					id={`bg_${hiddenInlineScript.index}`}
					bind:doNotRecomputeOnInputChanged={hiddenInlineScript.script.doNotRecomputeOnInputChanged}
					bind:inlineScript={hiddenInlineScript.script.inlineScript}
				/>
			{:else}
				<span class="text-gray-600 text-xs">No hiddenInlineScript.script defined</span>
			{/if}
		</div>
		{#if Object.keys(hiddenInlineScript.script.fields).length > 0}
			<PanelSection title={`Inputs`}>
				{#key $stateId}
					<InputsSpecsEditor
						id={`bg_${hiddenInlineScript.index}`}
						shouldCapitalize={false}
						bind:inputSpecs={hiddenInlineScript.script.fields}
						userInputEnabled={false}
					/>
				{/key}
			</PanelSection>
		{/if}
		<div class="grow shrink" />
	</div>
{/if}
