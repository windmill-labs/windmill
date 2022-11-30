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
	const COLS = 6

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

		const newItem: GridItem = {
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
				rounded-sm hover:bg-gray-100"
		>
			<h1 class="text-sm font-semibold text-left">{title}</h1>
			<Icon data={faAngleDown} class="rotate-0 duration-300 {isOpen ? '!rotate-180' : ''}" />
		</button>
		{#if isOpen}
			<div transition:slide|local={{ duration: 300 }}>
				{#if components.length}
					<div class="grid grid-cols-3 gap-1 p-2">
						{#each components as item, componentIndex (componentIndex)}
							<button
								on:click={() => addComponent(item, { w: 2, h: 2 })}
								class="border shadow-sm h-16 p-2 flex flex-col gap-2 items-center
									justify-center bg-white rounded-md scale-100 hover:scale-105 ease-in duration-75"
							>
								<Icon data={displayData[item.type].icon} scale={1.6} class="text-blue-800" />
								<div class="text-xs">{displayData[item.type].name}</div>
							</button>
						{/each}
					</div>
				{:else}
					<div class="text-xs text-gray-500 text-center py-1">
						There are no components in this group yet
					</div>
				{/if}
			</div>
		{/if}
	</section>
{/each}
