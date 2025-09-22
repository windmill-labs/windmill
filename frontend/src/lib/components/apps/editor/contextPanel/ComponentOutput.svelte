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

	interface Props {
		gridItem: GridItem
		first?: boolean
		nested?: boolean
		expanded?: boolean
		renderRec?: boolean
	}

	let {
		gridItem,
		first = false,
		nested = false,
		expanded = false,
		renderRec = true
	}: Props = $props()
	const { connectingInput } = getContext<AppViewerContext>('AppViewerContext')
	const name = getComponentNameById(gridItem.id)

	let nameOverrides = $derived(
		gridItem?.data?.type === 'decisiontreecomponent'
			? gridItem.data.nodes.map((n, i) => `${n.label} (Tab index ${i})`)
			: undefined
	)

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

	let subGrids = $derived(
		Array.from({ length: gridItem.data?.numberOfSubgrids ?? 0 }).map(
			(_, i) => `${gridItem.id}-${i}`
		)
	)
</script>

<OutputHeader
	render={renderRec}
	id={gridItem.id}
	name={getComponentNameById(gridItem.id)}
	{first}
	{nested}
>
	{#snippet children({ render })}
		<ComponentOutputViewer
			{render}
			componentId={gridItem.id}
			on:select={({ detail }) => {
				connectOutput(connectingInput, gridItem?.data?.type, gridItem.data.id, detail)
			}}
		/>
		<SubGridOutput {render} {name} {nameOverrides} {expanded} {subGrids} parentId={gridItem.id} />
		<TableActionsOutput {render} {gridItem} />
		<MenuItemsOutput {render} {gridItem} />
	{/snippet}
</OutputHeader>
