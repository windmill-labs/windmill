<script lang="ts">
	import { components } from '../component'
	import { getContext } from 'svelte'
	import type { AppViewerContext, GridItem } from '../../types'
	import ComponentOutputViewer from './ComponentOutputViewer.svelte'
	import { connectInput, isIdInsideGriditem } from '../appUtils'
	import SubGridOutput from './SubGridOutput.svelte'
	import OutputHeader from './components/OutputHeader.svelte'
	import TableActionsOutput from './components/TableActionsOutput.svelte'

	export let gridItem: GridItem
	export let first: boolean = false
	export let nested: boolean = false
	export let parentId: string | undefined = undefined
	export let expanded: boolean = false

	const { app, staticOutputs, selectedComponent, connectingInput } =
		getContext<AppViewerContext>('AppViewerContext')
	const name = getComponentNameById(gridItem.id)

	function getComponentNameById(componentId: string) {
		if (gridItem?.data?.type) {
			return components[gridItem?.data.type].name
		} else if (componentId == 'ctx') {
			return 'Context'
		} else if (componentId.startsWith('bg_')) {
			return 'Background'
		} else {
			return 'Table action'
		}
	}

	$: subGrids = Array.from({ length: gridItem.data.numberOfSubgrids }).map(
		(_, i) => `${gridItem.id}-${i}`
	)

	$: insideGrid = isIdInsideGriditem($app, gridItem, $selectedComponent)
	$: isSelected = $selectedComponent === gridItem.id
	$: shouldOpen = insideGrid || isSelected

	function onHeaderClick(manuallyOpen: boolean) {
		if (manuallyOpen) {
			if (parentId) {
				$selectedComponent = parentId
			} else {
				$selectedComponent = undefined
			}
		} else {
			$selectedComponent = gridItem.id
		}
	}
</script>

{#if $staticOutputs[gridItem.id] || gridItem.data.numberOfSubgrids > 1}
	<OutputHeader
		{shouldOpen}
		on:handleClick={(e) => {
			if (!$connectingInput.opened) {
				onHeaderClick(e.detail.manuallyOpen)
			}
		}}
		id={gridItem.id}
		name={getComponentNameById(gridItem.id)}
		{first}
		{nested}
		{expanded}
	>
		<div class="py-1">
			<ComponentOutputViewer
				componentId={gridItem.id}
				outputs={$staticOutputs[gridItem.id]}
				on:select={({ detail }) => {
					if ($connectingInput.opened) {
						$connectingInput = connectInput($connectingInput, gridItem.id, detail)
					}
				}}
			/>
		</div>

		<SubGridOutput {name} {expanded} {subGrids} parentId={gridItem.id} />
		<TableActionsOutput {gridItem} {expanded} />
	</OutputHeader>
{/if}
