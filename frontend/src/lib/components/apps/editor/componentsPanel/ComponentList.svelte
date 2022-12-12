<script lang="ts">
	import { slide } from 'svelte/transition'
	import Icon from 'svelte-awesome'
	import type { AppComponent, AppEditorContext, GridItem } from '../../types'
	import { displayData } from '../../utils'
	import { componentSets } from './data'

	import gridHelp from 'svelte-grid/build/helper/index.mjs'
	import { getContext, onMount } from 'svelte'
	import { getNextId } from '$lib/components/flows/flowStateUtils'
	import type { Size } from 'svelte-grid'
	import { faAngleDown } from '@fortawesome/free-solid-svg-icons'
	import { isOpenStore } from './store'
	import { gridColumns } from '../../gridUtils'

	const { app } = getContext<AppEditorContext>('AppEditorContext')

	function getMinDimensionsByComponent(componentType: string, column: number): Size {
		console.log(componentType, column)
		if (componentType === 'buttoncomponent') {
			return column === 3 ? { w: 1, h: 1 } : { w: 3, h: 1 }
		} else if (componentType === 'textcomponent') {
			return column === 3 ? { w: 1, h: 1 } : { w: 3, h: 1 }
		} else if (componentType === 'textinputcomponent') {
			return column === 3 ? { w: 1, h: 2 } : { w: 3, h: 2 }
		} else if (componentType === 'barchartcomponent') {
			return column === 3 ? { w: 2, h: 4 } : { w: 6, h: 4 }
		} else if (componentType === 'piechartcomponent') {
			return column === 3 ? { w: 2, h: 4 } : { w: 6, h: 4 }
		} else if (componentType === 'tablecomponent') {
			return column === 3 ? { w: 3, h: 4 } : { w: 12, h: 4 }
		} else if (componentType === 'displaycomponent') {
			return column === 3 ? { w: 2, h: 2 } : { w: 6, h: 4 }
		} else if (componentType === 'checkboxcomponent') {
			return column === 3 ? { w: 1, h: 1 } : { w: 3, h: 1 }
		} else {
			return { w: 2, h: 1 }
		}
	}

	function addComponent(appComponent: AppComponent) {
		const grid = $app.grid ?? []
		const id = getNextId(grid.map((gridItem) => gridItem.data.id))

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

		const newItem: GridItem = {
			data: JSON.parse(JSON.stringify(appComponent)),
			id: id
		}

		gridColumns.forEach((column) => {
			newItem[column] = newComponent
			const position = gridHelp.findSpace(newItem, grid, column)
			const min = getMinDimensionsByComponent(appComponent.type, column)

			const max = { w: 12, h: 12 }

			newItem[column].w = min.w
			newItem[column].h = min.h

			newItem[column] = { ...newItem[column], ...position, min, max }
		})

		$app.grid = [...grid, newItem]

		gridColumns.forEach((colIndex) => {
			$app.grid = gridHelp.adjust($app.grid, colIndex)
		})
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
					<div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-2 p-2">
						{#each components as item}
							<button
								on:click={() => addComponent(item)}
								title={displayData[item.type].name}
								class="border shadow-sm h-16 p-2 flex flex-col gap-2 items-center
									justify-center bg-white rounded-md scale-100 hover:scale-105 ease-in duration-75"
							>
								<svelte:component this={displayData[item.type].icon} class="text-blue-800" />
								<div class="text-xs w-full text-center ellipsize">
									{displayData[item.type].name}
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
