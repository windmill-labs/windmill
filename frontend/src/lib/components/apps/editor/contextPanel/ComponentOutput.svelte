<script lang="ts">
	import { components } from '../component'
	import { getContext } from 'svelte'
	import type { AppViewerContext, GridItem } from '../../types'
	import ComponentOutputViewer from './ComponentOutputViewer.svelte'
	import { classNames } from '$lib/utils'
	import { ChevronDown, ChevronRight, FolderOpen } from 'lucide-svelte'
	import { isIdInsideGriditem } from '../appUtils'
	import { slide } from 'svelte/transition'
	import SubGridOutput from './SubGridOutput.svelte'

	export let gridItem: GridItem
	export let first: boolean = false
	export let nested: boolean = false
	export let parentId: string | undefined = undefined
	export let expanded: boolean = false

	const { app, staticOutputs, selectedComponent } = getContext<AppViewerContext>('AppViewerContext')
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
	<!-- svelte-ignore a11y-click-events-have-key-events -->
	<div
		class={classNames(
			'flex items-center justify-between p-1 cursor-pointer hover:bg-indigo-100 hover:text-indigo-500 border-b',
			isSelected ? 'bg-indigo-200' : 'bg-white',
			first ? 'border-t' : '',
			nested ? 'border-l' : ''
		)}
		on:click={onHeaderClick}
	>
		<div
			class={classNames(
				'text-2xs ml-0.5 font-bold px-2 py-0.5 rounded-sm',
				isSelected ? 'bg-indigo-500 text-white' : ' bg-indigo-50'
			)}
		>
			{gridItem.id}
		</div>
		<div class="text-2xs font-bold flex flex-row gap-2 items-center">
			{getComponentNameById(gridItem.id)}
			{#if !opened && !manuallyOpened}
				<ChevronRight size={14} />
			{:else if manuallyOpened}
				<FolderOpen size={14} />
			{:else}
				<ChevronDown size={14} />
			{/if}
		</div>
	</div>

	{#if opened}
		<div class={classNames('border-b', nested ? 'border-l' : '')} transition:slide|local>
			<div class={classNames('py-1')}>
				<ComponentOutputViewer componentId={gridItem.id} outputs={$staticOutputs[gridItem.id]} />
			</div>

			<div>
				<SubGridOutput {name} {expanded} {subGrids} parentId={gridItem.id} />
			</div>
		</div>
	{/if}
{/if}
