<script lang="ts">
	import { components } from '../component'
	import { getContext } from 'svelte'
	import type { AppViewerContext, GridItem } from '../../types'
	import ComponentOutputViewer from './ComponentOutputViewer.svelte'
	import { classNames, pluralize } from '$lib/utils'
	import { ArrowDown, ArrowRight, ChevronDown, ChevronRight } from 'lucide-svelte'
	import { isIdInsideGriditem } from '../appUtils'

	export let gridItem: GridItem
	export let nested: boolean = false

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

	$: subGrids = Array.from({ length: gridItem.data.numberOfSubgrids }).map(
		(_, i) => `${gridItem.id}-${i}`
	)

	function getSubgridName(name: string) {
		if (name === 'Tabs') {
			return 'Tab'
		} else if (name === 'Horizontal Split Panes') {
			return 'Pane'
		} else if (name === 'Vertical Split Panes') {
			return 'Pane'
		}
	}
	$: x = isIdInsideGriditem($app, gridItem, $selectedComponent)

	$: opened = manuallyOpened || x
</script>

{#if $staticOutputs[gridItem.id] || gridItem.data.numberOfSubgrids > 1}
	<div class="flex flex-col">
		<div
			class={classNames(
				'flex items-center justify-between p-1 border-y',
				opened ? 'bg-gray-200' : 'bg-gray-50'
			)}
		>
			<div
				class={classNames(
					'text-2xs ml-0.5 font-bold px-2 py-0.5',
					$selectedComponent === gridItem.id ? 'bg-indigo-500 text-white' : ' bg-indigo-200'
				)}
			>
				{gridItem.id}
			</div>
			<div class="text-xs font-bold flex flex-row gap-2 items-center">
				{getComponentNameById(gridItem.id)}
				{#if !opened}
					<ChevronRight size={14} />
				{:else}
					<ChevronDown size={14} />
				{/if}
			</div>
		</div>
		{#if x || opened}
			<div class="py-1">
				<ComponentOutputViewer componentId={gridItem.id} outputs={$staticOutputs[gridItem.id]} />
			</div>

			<div class={nested ? 'border-b' : ''}>
				{#each subGrids as subGridId, index}
					<div class="ml-2 mb-2">
						{#if subGrids.length > 1}
							<div
								class="px-1 py-0.5 flex justify-between items-center font-semibold bg-gray-600 text-xs text-white w-fit	 "
							>
								<div class="text-xs">
									{getSubgridName(name)}
									{index + 1}
								</div>
							</div>
						{/if}
						{#if $app.subgrids && $app.subgrids[subGridId].length > 0}
							{#each $app.subgrids[subGridId] as subGridItem}
								{#if subGridItem}
									<div class="border-l">
										<svelte:self gridItem={subGridItem} nested />
									</div>
								{/if}
							{/each}
						{:else}
							<div class="text-xs text-gray-500 border-y border-l p-1">No components</div>
						{/if}
					</div>
				{/each}
			</div>
		{/if}
	</div>
{/if}
