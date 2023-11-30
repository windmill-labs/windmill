<script lang="ts">
	import { components } from '../component'
	import { getContext } from 'svelte'
	import type { AppViewerContext, GridItem } from '../../types'
	import ComponentOutputViewer from './ComponentOutputViewer.svelte'
	import { connectOutput } from '../appUtils'
	import SubGridOutput from './SubGridOutput.svelte'
	import OutputHeader from './components/OutputHeader.svelte'
	import TableActionsOutput from './components/TableActionsOutput.svelte'
	import { BG_PREFIX } from '../../utils'
	// @ts-ignore
	import MenuItemsOutput from './components/MenuItemsOutput.svelte'

	export let gridItem: GridItem
	export let first: boolean = false
	export let nested: boolean = false
	export let expanded: boolean = false

	const { connectingInput } = getContext<AppViewerContext>('AppViewerContext')
	const name = getComponentNameById(gridItem.id)

	function getComponentNameById(componentId: string) {
		if (gridItem?.data?.type) {
			return components?.[gridItem?.data.type]?.name ?? 'Unknown'
		} else if (componentId == 'ctx') {
			return 'Context'
		} else if (componentId.startsWith(BG_PREFIX)) {
			return 'Background'
		} else {
			return 'Table action'
		}
	}

	$: subGrids = Array.from({ length: gridItem.data.numberOfSubgrids ?? 0 }).map(
		(_, i) => `${gridItem.id}-${i}`
	)
</script>

<OutputHeader id={gridItem.id} name={getComponentNameById(gridItem.id)} {first} {nested}>
	<ComponentOutputViewer
		componentId={gridItem.id}
		on:select={({ detail }) => {
			connectOutput(connectingInput, gridItem?.data?.type, gridItem.data.id, detail)
		}}
	/>
	<SubGridOutput {name} {expanded} {subGrids} parentId={gridItem.id} />
	<TableActionsOutput {gridItem} />
	<MenuItemsOutput {gridItem} />
</OutputHeader>
