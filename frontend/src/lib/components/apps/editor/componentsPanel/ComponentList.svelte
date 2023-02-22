<script lang="ts">
	import type { AppEditorContext, GridItem } from '../../types'
	import gridHelp from 'svelte-grid/build/helper/index.mjs'
	import { getContext, onMount } from 'svelte'
	import { getNextId } from '$lib/components/flows/flowStateUtils'
	import { isOpenStore } from './store'
	import { gridColumns } from '../../gridUtils'
	import { dirtyStore } from '$lib/components/common/confirmationModal/dirtyStore'
	import {
		components as componentsRecord,
		componentSets,
		getRecommendedDimensionsByComponent,
		type AppComponent
	} from '../Component.svelte'
	import ListItem from './ListItem.svelte'

	const TITLE_PREFIX = 'Component.' as const
	const { app, selectedComponent } = getContext<AppEditorContext>('AppEditorContext')

	function addComponent(appComponentType: AppComponent['type']) {
		$dirtyStore = true
		const grid = $app.grid ?? []
		const id = getNextId(grid.map((gridItem) => gridItem.data.id))

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

		$app.grid = [...grid, newItem]

		$selectedComponent = id
	}

	onMount(() => {
		isOpenStore.addItems(componentSets.map((set) => ({ [TITLE_PREFIX + set.title]: true })))
	})
</script>

{#each componentSets as { title, components }, index (index)}
	<ListItem {title} prefix={TITLE_PREFIX}>
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
	</ListItem>
{/each}
