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
	import { dirtyStore } from '$lib/components/common/confirmationModal/dirtyStore'

	const { app, selectedComponent } = getContext<AppEditorContext>('AppEditorContext')

	function getRecommendedDimensionsByComponent(
		componentType: AppComponent['type'],
		column: number
	): Size {
		// Dimensions key formula: <mobile width>:<mobile height>-<desktop width>:<desktop height>
		const dimensions: Record<`${number}:${number}-${number}:${number}`, AppComponent['type'][]> = {
			'1:1-3:1': ['textcomponent'],
			'1:1-2:1': ['buttoncomponent', 'checkboxcomponent'],
			'1:2-1:2': ['htmlcomponent'],
			'2:1-3:1': [
				'textinputcomponent',
				'numberinputcomponent',
				'selectcomponent',
				'passwordinputcomponent',
				'dateinputcomponent'
			],
			'3:5-6:5': ['formcomponent'],
			'2:8-6:8': [
				'timeseriescomponent',
				'barchartcomponent',
				'piechartcomponent',
				'displaycomponent',
				'scatterchartcomponent',
				'vegalitecomponent'
			],
			'3:10-6:10': ['tablecomponent']
		}
		// Finds the key that is associated with the component type and extracts the dimensions from it
		const [dimension] = Object.entries(dimensions).find(([_, value]) =>
			value.includes(componentType)
		) || ['2:1-2:1']

		const size = dimension.split('-')[column === 3 ? 0 : 1].split(':')
		return { w: +size[0], h: +size[1] }
	}

	function addComponent(appComponent: AppComponent) {
		$dirtyStore = true
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
								title={displayData[item.type].name}
								class="border w-24 shadow-sm h-16 p-2 flex flex-col gap-2 items-center
									justify-center bg-white rounded-md hover:bg-gray-100 duration-200"
							>
								<svelte:component this={displayData[item.type].icon} />
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
