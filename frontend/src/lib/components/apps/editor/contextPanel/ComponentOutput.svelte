<script lang="ts">
	import { components } from '../component'
	import { getContext } from 'svelte'
	import type { AppViewerContext, GridItem } from '../../types'
	import ComponentOutputViewer from './ComponentOutputViewer.svelte'
	import { classNames } from '$lib/utils'
	import { connectInput, isIdInsideGriditem } from '../appUtils'
	import { slide } from 'svelte/transition'
	import SubGridOutput from './SubGridOutput.svelte'
	import OutputHeader from './components/OutputHeader.svelte'

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

	let manuallyOpened = false

	$: if (expanded) {
		manuallyOpened = true
	} else {
		manuallyOpened = false
	}

	$: subGrids = Array.from({ length: gridItem.data.numberOfSubgrids }).map(
		(_, i) => `${gridItem.id}-${i}`
	)

	$: insideGrid = isIdInsideGriditem($app, gridItem, $selectedComponent)
	$: isSelected = $selectedComponent === gridItem.id
	$: opened = insideGrid || isSelected || manuallyOpened

	function onHeaderClick() {
		if (manuallyOpened) {
			if (parentId) {
				$selectedComponent = parentId
			} else {
				$selectedComponent = undefined
			}
			manuallyOpened = false
		} else {
			$selectedComponent = gridItem.id
			manuallyOpened = true
		}
	}
</script>

{#if $staticOutputs[gridItem.id] || gridItem.data.numberOfSubgrids > 1}
	<OutputHeader
		open={opened}
		manuallyOpen={manuallyOpened}
		on:click={onHeaderClick}
		id={gridItem.id}
		name={getComponentNameById(gridItem.id)}
		{first}
		{nested}
	/>

	{#if opened}
		<div class={classNames('border-b', nested ? 'border-l' : '')} transition:slide|local>
			<div class="py-1">
				<ComponentOutputViewer
					componentId={gridItem.id}
					outputs={$staticOutputs[gridItem.id]}
					on:select={({ detail }) => {
						$connectingInput = connectInput($connectingInput, gridItem.id, detail)
					}}
				/>
			</div>

			<div>
				<SubGridOutput {name} {expanded} {subGrids} parentId={gridItem.id} />
			</div>
		</div>
	{/if}
{/if}
