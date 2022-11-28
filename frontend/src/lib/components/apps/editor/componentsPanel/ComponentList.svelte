<script lang="ts">
	import Icon from 'svelte-awesome'
	import type { AppComponent, AppEditorContext, GridItem } from '../../types'
	import { displayData } from '../../utils'
	import { componentSets } from './data'

	import gridHelp from 'svelte-grid/build/helper/index.mjs'
	import { getContext } from 'svelte'
	import { getNextId } from '$lib/components/flows/flowStateUtils'
	import type { Size } from 'svelte-grid'
	import { gridColumns } from '../../gridUtils'

	const { app } = getContext<AppEditorContext>('AppEditorContext')

	function addComponent(
		appComponent: AppComponent,
		defaultDimensions: Size,
		minDimensions: Size = { w: 2, h: 1 },
		maxDimensions: Size = { w: 6, h: 12 }
	) {
		const grid = $app.grid ?? []
		const id = getNextId(grid.map((gridItem) => gridItem.data.id))

		appComponent.id = id

		const newComponent = {
			fixed: false,
			resizable: true,
			draggable: true,
			customDragger: false,
			customResizer: false,
			min: minDimensions,
			max: maxDimensions,
			x: 0,
			y: 0,
			...defaultDimensions
		}

		let newItem: GridItem = {
			data: JSON.parse(JSON.stringify(appComponent)),
			id: id
		}

		gridColumns.forEach((column) => {
			newItem[column] = newComponent
			const position = gridHelp.findSpace(newItem, grid, column)
			newItem[column] = { ...newItem[column], ...position }
		})

		$app.grid = [...grid, newItem]
	}
</script>

{#each componentSets as componentSet, index (index)}
	<div class="px-4 pt-4 text-sm font-semibold">{componentSet.title}</div>

	<section class="grid grid-cols-3 gap-1 p-4">
		{#each componentSet.components as item, componentIndex (componentIndex)}
			<!-- svelte-ignore a11y-click-events-have-key-events -->
			<div
				class="border shadow-sm h-16 p-2 flex flex-col gap-2 items-center justify-center bg-white rounded-md scale-100 hover:scale-105 ease-in duration-75"
				on:click={() => addComponent(item, { w: 2, h: 1 })}
			>
				<Icon data={displayData[item.type].icon} scale={1.6} class="text-blue-800" />
				<div class="text-xs">{displayData[item.type].name}</div>
			</div>
		{/each}
	</section>
{/each}
