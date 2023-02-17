<script lang="ts">
	import { slide } from 'svelte/transition'
	import Icon from 'svelte-awesome'
	import type { AppEditorContext, GridItem } from '../../types'

	import gridHelp from 'svelte-grid/build/helper/index.mjs'
	import { getContext, onMount } from 'svelte'
	import { getNextId } from '$lib/components/flows/flowStateUtils'
	import { faAngleDown } from '@fortawesome/free-solid-svg-icons'
	import { isOpenStore } from './store'
	import { gridColumns } from '../../gridUtils'
	import { dirtyStore } from '$lib/components/common/confirmationModal/dirtyStore'
	import {
		components as componentsRecord,
		componentSets,
		getRecommendedDimensionsByComponent,
		type AppComponent
	} from '../Component.svelte'

	const { app, selectedComponent, focusedGrid } = getContext<AppEditorContext>('AppEditorContext')

	// The grid is needed to find a space for the new component
	function createNewGridItem(grid: GridItem[], id: string, appComponentType: string): GridItem {
		const appComponent = componentsRecord[appComponentType].data

		appComponent.id = id

		const newComponent = {
			fixed: false,
			resizable: true,
			draggable: true,
			customDragger: false,
			customResizer: false,
			x: 0,
			y: 0
		}

		let newData: AppComponent = JSON.parse(JSON.stringify(appComponent))

		const newItem: GridItem = {
			data: newData,
			id: id
		}

		gridColumns.forEach((column) => {
			const rec = getRecommendedDimensionsByComponent(appComponent.type, column)

			newItem[column] = {
				...newComponent,
				min: { w: 1, h: 1 },
				max: { w: column, h: 100 },
				w: rec.w,
				h: rec.h
			}
			const position = gridHelp.findSpace(newItem, grid, column) as { x: number; y: number }
			newItem[column] = { ...newItem[column], ...position }
		})

		return newItem
	}

	function addComponent(appComponentType: AppComponent['type']): void {
		// When a new component is added, we need to mark the app as dirty,
		// so a confirmation modal will appear if the user tries to leave the page
		$dirtyStore = true

		const grid = $app.grid ?? []

		const gridItemIds = grid
			.map((gridItem: GridItem) => {
				const subGrids = gridItem.data.subGrids ?? []
				return [
					gridItem.data.id,
					...subGrids.map((subGrid: GridItem[]) =>
						subGrid.map((gridItem: GridItem) => gridItem.data.id)
					)
				]
			})
			.flat(2)

		const id = getNextId(gridItemIds)

		if ($focusedGrid) {
			const { parentComponentId, subGridIndex } = $focusedGrid

			const gridItemIndex = $app.grid.findIndex((gridItem) => gridItem.id === parentComponentId)
			const subGrids = $app.grid[gridItemIndex].data.subGrids ?? []
			const newItem = createNewGridItem(subGrids[subGridIndex] ?? [], id, appComponentType)
			const subGrid = subGrids[subGridIndex] ?? []

			$app.grid[gridItemIndex].data.subGrids[subGridIndex] = [...subGrid, newItem]
			$selectedComponent = id
		} else {
			const newItem = createNewGridItem(grid, id, appComponentType)
			$app.grid = [...grid, newItem]
			$selectedComponent = id
		}
	}

	onMount(() => {
		isOpenStore.addItems(componentSets.map((set) => ({ [set.title]: true })))
	})
</script>

{#each componentSets as { title, components }, index (index)}
	{@const isOpen = $isOpenStore[title]}
	<section class="mt-1 mb-2 px-2">
		<button
			on:click|preventDefault={() => isOpenStore.toggle(title)}
			class="w-full flex justify-between items-center text-gray-700 px-2 py-1 
				rounded-sm duration-200 hover:bg-gray-100"
		>
			<h1 class="text-sm font-semibold text-left">{title}</h1>
			<Icon data={faAngleDown} class="rotate-0 duration-300 {isOpen ? '!rotate-180' : ''}" />
		</button>
		{#if isOpen}
			<div transition:slide|local={{ duration: 300 }}>
				{#if components.length}
					<div class="flex flex-wrap gap-2 py-2">
						{#each components as item}
							<button
								on:click={() => addComponent(item)}
								title={componentsRecord[item].name}
								class="border w-24 shadow-sm h-16 p-2 flex flex-col gap-2 items-center
									justify-center bg-white rounded-md hover:bg-gray-100 duration-200"
							>
								<svelte:component this={componentsRecord[item].icon} />
								<div class="text-xs w-full text-center ellipsize">
									{componentsRecord[item].name}
								</div>
							</button>
						{/each}
					</div>
				{:else}
					<div class="text-xs text-gray-500 py-1 px-2">
						There are no components in this group yet
					</div>
				{/if}
			</div>
		{/if}
	</section>
{/each}
