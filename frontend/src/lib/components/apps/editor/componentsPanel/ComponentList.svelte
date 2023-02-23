<script lang="ts">
	import type { AppEditorContext, GridItem } from '../../types'
	import { getContext, onMount } from 'svelte'
	import { getNextId } from '$lib/components/flows/flowStateUtils'
	import { isOpenStore } from './store'
	import { dirtyStore } from '$lib/components/common/confirmationModal/dirtyStore'
	import { components as componentsRecord, COMPONENT_SETS, type AppComponent } from '../component'
	import ListItem from './ListItem.svelte'
	import { insertNewGridItem, createNewGridItem, getNextGridItemId } from '../../utils'

	const TITLE_PREFIX = 'Component.' as const
	const { app, selectedComponent, focusedGrid } = getContext<AppEditorContext>('AppEditorContext')

	function addComponent(appComponentType: AppComponent['type']): void {
		// When a new component is added, we need to mark the app as dirty,
		// so a confirmation modal will appear if the user tries to leave the page
		$dirtyStore = true

		const grid = $app.grid ?? []
		const id = getNextGridItemId(grid)

		const data = componentsRecord[appComponentType].data

		if ($focusedGrid) {
			const { parentComponentId, subGridIndex } = $focusedGrid

			$app.grid = insertNewGridItem($app.grid, parentComponentId, subGridIndex, id, data)
		} else {
			const newItem = createNewGridItem(grid, id, data)
			$app.grid = [...grid, newItem]
		}

		$selectedComponent = id
	}

	onMount(() => {
		isOpenStore.addItems(COMPONENT_SETS.map((set) => ({ [TITLE_PREFIX + set.title]: true })))
	})
</script>

{#each COMPONENT_SETS as { title, components }, index (index)}
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
			<div class="text-xs text-gray-500 py-1 px-2"> There are no components in this group yet </div>
		{/if}
	</ListItem>
{/each}
