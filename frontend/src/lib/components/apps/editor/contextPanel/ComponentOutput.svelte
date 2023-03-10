<script lang="ts">
	import { components } from '../component'
	import { getContext } from 'svelte'
	import type { AppEditorContext, GridItem } from '../../types'
	import ComponentOutputViewer from './ComponentOutputViewer.svelte'

	export let gridItem: GridItem

	const { app, staticOutputs, selectedComponent } = getContext<AppEditorContext>('AppEditorContext')
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

	function x(name: string) {
		if (name === 'Tabs') {
			return 'Tab'
		} else if (name === 'Horizontal Split Panes') {
			return 'Pane'
		}
	}
</script>

{#if $staticOutputs[gridItem.id] || gridItem.data.numberOfSubgrids > 1}
	<div class="flex flex-col  mb-2">
		<div class="flex items-center justify-between p-1 border-t border-l border-b bg-gray-100">
			<div class="text-xs ml-1 bg-indigo-500  text-white px-2 py-0.5">
				{gridItem.id}
			</div>
			<div class="text-xs font-bold">
				{getComponentNameById(gridItem.id)}
			</div>
		</div>
		{#if $selectedComponent === gridItem.id}
			<div class="my-1">
				<ComponentOutputViewer componentId={gridItem.id} outputs={$staticOutputs[gridItem.id]} />
			</div>

			<div>
				{#each subGrids as subGridId, index}
					{#if subGrids.length > 1}
						<div class="ml-2 border-y border-l bg-gray-400 font-bold text-xs p-1 my-1">
							{x(name)}
							{index + 1}

							{subGridId}
						</div>
					{/if}
					{#if $app.subgrids}
						{#each $app.subgrids[subGridId] as subGridItem}
							{#if subGridItem}
								<div class="ml-2 pl-2">
									<svelte:self gridItem={subGridItem} />
								</div>
							{/if}
						{/each}
					{/if}
				{/each}
			</div>
		{/if}
	</div>
{/if}
