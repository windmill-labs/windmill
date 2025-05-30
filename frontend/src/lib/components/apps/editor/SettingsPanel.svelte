<script lang="ts">
	import { createEventDispatcher, getContext } from 'svelte'
	import type { App, AppViewerContext, GridItem } from '../types'
	import { BG_PREFIX } from '../utils'
	import { findGridItemWithLocation, allItemsWithLocation } from './appUtils'
	import PanelSection from './settingsPanel/common/PanelSection.svelte'
	import ComponentPanel from './settingsPanel/ComponentPanel.svelte'
	import InputsSpecsEditor from './settingsPanel/InputsSpecsEditor.svelte'
	import BackgroundScriptSettings from './settingsPanel/script/BackgroundScriptSettings.svelte'
	import EventHandlerItem from './settingsPanel/EventHandlerItem.svelte'
	import type { TableAction } from './component'

	const { selectedComponent, app, stateId, runnableComponents } =
		getContext<AppViewerContext>('AppViewerContext')

	let firstComponent = $selectedComponent?.[0]

	$: $selectedComponent?.[0] != firstComponent && (firstComponent = $selectedComponent?.[0])

	$: hiddenInlineScript = $app?.hiddenInlineScripts
		?.map((x, i) => ({ script: x, index: i }))
		.find(({ script, index }) => $selectedComponent?.includes(BG_PREFIX + index))

	$: gridItemWithLocation = findGridItemWithLocation($app, firstComponent)
	$: tableActionSettings = findTableActionSettings($app, firstComponent)
	$: menuItemsSettings = findMenuItemsSettings($app, firstComponent)

	function findTableActionSettings(app: App, id: string | undefined) {
		return allItemsWithLocation(app.grid, app.subgrids)
			.map((itemWithLocation) => {
				const x = itemWithLocation.item
				if (x?.data?.type === 'tablecomponent') {
					if (x?.data?.actionButtons) {
						const tableActionIdx = x.data.actionButtons.findIndex((x) => x.id === id)
						if (tableActionIdx > -1) {
							const tableAction = x.data.actionButtons[tableActionIdx]
							return {
								item: { data: tableAction, id: tableAction.id },
								parent: x.data.id,
								gridItemLocation: itemWithLocation.location,
								location: {
									key: 'actionButtons',
									index: tableActionIdx
								}
							}
						}
					}
				} else if (
					x?.data?.type === 'aggridcomponent' ||
					x?.data?.type === 'aggridcomponentee' ||
					x?.data?.type === 'dbexplorercomponent' ||
					x?.data?.type === 'aggridinfinitecomponent' ||
					x?.data?.type === 'aggridinfinitecomponentee'
				) {
					if (x?.data?.actions) {
						const tableActionIdx = x.data.actions.findIndex((x) => x.id === id)
						if (tableActionIdx > -1) {
							const tableAction = x.data.actions[tableActionIdx]
							return {
								item: { data: tableAction, id: tableAction.id },
								parent: x.data.id,
								gridItemLocation: itemWithLocation.location,
								location: {
									key: 'actions',
									index: tableActionIdx
								}
							}
						}
					}
				}
			})
			.find((x) => x)
	}

	function findMenuItemsSettings(app: App, id: string | undefined) {
		return allItemsWithLocation(app.grid, app.subgrids)
			.map((itemWithLocation) => {
				const x = itemWithLocation.item
				if (x?.data?.type === 'menucomponent') {
					if (x?.data?.menuItems) {
						const menuItemIdx = x.data.menuItems.findIndex((x) => x.id === id)
						if (menuItemIdx > -1) {
							const menuItem = x.data.menuItems[menuItemIdx]
							return {
								item: { data: menuItem, id: menuItem.id },
								parent: x.data.id,
								index: menuItemIdx,
								gridItemLocation: itemWithLocation.location
							}
						}
					}
				}
			})
			.find((x) => x)
	}

	function itemHasActions(
		item: GridItem | undefined
	): item is GridItem & { data: { actions: TableAction[] } } {
		return (
			item?.data?.type === 'aggridcomponent' ||
			item?.data?.type === 'aggridcomponentee' ||
			item?.data?.type === 'dbexplorercomponent' ||
			item?.data?.type === 'aggridinfinitecomponent' ||
			item?.data?.type === 'aggridinfinitecomponentee'
		)
	}

	const dispatch = createEventDispatcher()
</script>

{#if gridItemWithLocation}
	{#key gridItemWithLocation.item.id}
		<ComponentPanel
			bind:componentSettings={
				() => gridItemWithLocation,
				(cs) => {
					if (gridItemWithLocation?.location.type === 'grid') {
						$app.grid[gridItemWithLocation.location.gridItemIndex] = cs.item
					} else if (
						gridItemWithLocation?.location.type === 'subgrid' &&
						Array.isArray($app.subgrids?.[gridItemWithLocation.location.subgridKey])
					) {
						if (
							$app.subgrids[gridItemWithLocation.location.subgridKey][
								gridItemWithLocation.location.subgridItemIndex
							]
						) {
							$app.subgrids[gridItemWithLocation.location.subgridKey][
								gridItemWithLocation.location.subgridItemIndex
							] = cs.item
						}
					}
				}
			}
			onDelete={() => {
				dispatch('delete')
			}}
		/>
	{/key}
{:else if tableActionSettings}
	{#key tableActionSettings?.item?.data?.id}
		<ComponentPanel
			noGrid
			bind:componentSettings={
				() => tableActionSettings,
				(cs) => {
					if (tableActionSettings) {
						if (tableActionSettings.gridItemLocation.type === 'grid') {
							const { gridItemIndex } = tableActionSettings.gridItemLocation
							const { key, index } = tableActionSettings.location
							if ($app.grid[gridItemIndex]?.data?.[key]) {
								$app.grid[gridItemIndex].data[key][index] = cs.item.data
							}
						} else if (tableActionSettings.gridItemLocation.type === 'subgrid') {
							const { subgridKey, subgridItemIndex } = tableActionSettings.gridItemLocation
							const { key, index } = tableActionSettings.location
							if ($app.subgrids?.[subgridKey]?.[subgridItemIndex]?.data?.[key]) {
								$app.subgrids[subgridKey][subgridItemIndex].data[key][index] = cs.item.data
							}
						}
					}
				}
			}
			duplicateMoveAllowed={false}
			onDelete={() => {
				if (tableActionSettings) {
					const item = findGridItemWithLocation($app, tableActionSettings.parent)
					if (!item) return
					const { item: parent, location } = item
					if (parent.data.type === 'tablecomponent') {
						const newActionButtons = parent.data.actionButtons.filter(
							(x) => x.id !== tableActionSettings?.item.id
						)
						if (location.type === 'grid') {
							const { gridItemIndex } = location
							if ($app.grid[gridItemIndex]?.data?.type === 'tablecomponent') {
								$app.grid[gridItemIndex].data.actionButtons = newActionButtons
							}
						} else if (location.type === 'subgrid') {
							const { subgridKey, subgridItemIndex } = location
							if (
								$app.subgrids?.[subgridKey]?.[subgridItemIndex]?.data?.type === 'tablecomponent'
							) {
								$app.subgrids[subgridKey][subgridItemIndex].data.actionButtons = newActionButtons
							}
						}
					}
					if (itemHasActions(parent) && Array.isArray(parent.data.actions)) {
						const newActions = parent.data.actions.filter(
							(x) => x.id !== tableActionSettings?.item.id
						)
						if (location.type === 'grid') {
							const { gridItemIndex } = location
							if (itemHasActions($app.grid[gridItemIndex])) {
								$app.grid[gridItemIndex].data.actions = newActions
							}
						} else {
							const { subgridKey, subgridItemIndex } = location
							if (itemHasActions($app.subgrids?.[subgridKey]?.[subgridItemIndex])) {
								$app.subgrids[subgridKey][subgridItemIndex].data.actions = newActions
							}
						}
					}
				}
			}}
		/>
	{/key}
{:else if menuItemsSettings}
	{#key menuItemsSettings?.item?.id}
		<ComponentPanel
			noGrid
			bind:componentSettings={
				() => menuItemsSettings,
				(cs) => {
					if (menuItemsSettings) {
						if (menuItemsSettings.gridItemLocation.type === 'grid') {
							const { gridItemIndex } = menuItemsSettings.gridItemLocation
							if ($app.grid[gridItemIndex]?.data?.type === 'menucomponent') {
								$app.grid[gridItemIndex].data.menuItems[cs.index] = cs.item.data
							}
						} else if (menuItemsSettings.gridItemLocation.type === 'subgrid') {
							const { subgridKey, subgridItemIndex } = menuItemsSettings.gridItemLocation
							if ($app.subgrids?.[subgridKey]?.[subgridItemIndex]?.data?.type === 'menucomponent') {
								$app.subgrids[subgridKey][subgridItemIndex].data.menuItems[cs.index] = cs.item.data
							}
						}
					}
				}
			}
			onDelete={() => {
				if (menuItemsSettings) {
					const item = findGridItemWithLocation($app, menuItemsSettings.parent)
					if (!item) return
					const { item: parent, location } = item
					if (parent.data.type === 'menucomponent') {
						const newItems = parent.data.menuItems.filter(
							(x) => x.id !== menuItemsSettings?.item.id
						)
						if (location.type === 'grid') {
							const { gridItemIndex } = location
							if ($app.grid[gridItemIndex]?.data?.type === 'menucomponent') {
								$app.grid[gridItemIndex].data.menuItems = newItems
							}
						} else if (location.type === 'subgrid') {
							const { subgridKey, subgridItemIndex } = location
							if ($app.subgrids?.[subgridKey]?.[subgridItemIndex]?.data?.type === 'menucomponent') {
								$app.subgrids[subgridKey][subgridItemIndex].data.menuItems = newItems
							}
						}
					}
				}
			}}
		/>
	{/key}
{:else if hiddenInlineScript}
	{@const id = BG_PREFIX + hiddenInlineScript.index}
	{#key id}
		<BackgroundScriptSettings
			bind:runnable={
				() => hiddenInlineScript.script,
				(r) => {
					$app.hiddenInlineScripts[hiddenInlineScript.index] = r
				}
			}
			{id}
		/>
		{#if Object.keys(hiddenInlineScript.script.fields ?? {}).length > 0}
			<div class="mb-8">
				<PanelSection title={`Inputs`}>
					{#key $stateId}
						<InputsSpecsEditor
							displayType
							{id}
							shouldCapitalize={false}
							bind:inputSpecs={
								() => hiddenInlineScript.script.fields,
								(is) => {
									if ($app.hiddenInlineScripts[hiddenInlineScript.index]) {
										$app.hiddenInlineScripts[hiddenInlineScript.index].fields = is
									}
								}
							}
							userInputEnabled={false}
							recomputeOnInputChanged={hiddenInlineScript.script.recomputeOnInputChanged}
							showOnDemandOnlyToggle
						/>
					{/key}
				</PanelSection>
			</div>
		{/if}
		<PanelSection
			title={`Event handlers`}
			fullHeight={false}
			tooltip="Event handlers are used to trigger actions on other components when a specific event occurs. For example, you can trigger a recompute on a component when a script has successfully run."
		>
			<EventHandlerItem
				title="on success"
				tooltip="This event is triggered when the script runs successfully."
				items={Object.keys($runnableComponents).filter((_id) => _id !== id)}
				bind:value={
					() => hiddenInlineScript.script.recomputeIds ?? [],
					(v) => {
						if ($app.hiddenInlineScripts[hiddenInlineScript.index]) {
							$app.hiddenInlineScripts[hiddenInlineScript.index].recomputeIds = v
						}
					}
				}
			/>
		</PanelSection>
		<div class="grow shrink"></div>
	{/key}
{/if}
