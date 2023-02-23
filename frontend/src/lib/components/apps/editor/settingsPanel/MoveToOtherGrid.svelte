<script lang="ts">
	import { Button } from '$lib/components/common'
	import { dirtyStore } from '$lib/components/common/confirmationModal/dirtyStore'
	import { faCopy } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import type { AppEditorContext, FocusedGrid, GridItem } from '../../types'
	import { createNewGridItem, deleteNodeById, findNodeById, insertNewGridItem } from '../../utils'
	import type { AppComponent } from '../component'

	export let component: AppComponent | undefined

	const { app, selectedComponent } = getContext<AppEditorContext>('AppEditorContext')

	function listAllSubGrids(root: GridItem[]): FocusedGrid[] {
		const subGrids: FocusedGrid[] = []

		function findSubGrids(gridItem: GridItem): void {
			if (gridItem.data.subGrids) {
				for (const subGrid of gridItem.data.subGrids) {
					if (gridItem.id !== component?.id) {
						subGrids.push({
							parentComponentId: gridItem.id,
							subGridIndex: gridItem.data.subGrids.indexOf(subGrid)
						})
					}

					subGrid.forEach(findSubGrids)
				}
			}
		}

		root.forEach(findSubGrids)
		return subGrids.flat()
	}

	function moveComponent(id: string, targetId: string, targetSubGridIndex: number) {
		const node = findNodeById($app.grid, id)

		if (!node) {
			return
		}

		const data: AppComponent = JSON.parse(JSON.stringify(node.data))
		$dirtyStore = true

		deleteNodeById($app.grid, id)
		$app.grid = insertNewGridItem($app.grid, targetId, targetSubGridIndex, id, data)

		$selectedComponent = id
	}

	$: availableGrids = listAllSubGrids($app.grid)

	function onMove() {
		if (component) {
			const [targetId, targetSubGridIndex] = (
				document.getElementById('move-to-grid') as HTMLSelectElement
			).value.split(':')

			if (targetId !== 'main' && targetSubGridIndex !== 'grid') {
				moveComponent(component.id, targetId, parseInt(targetSubGridIndex))
			} else {
				const node = findNodeById($app.grid, component.id)

				if (!node) {
					return
				}

				const data: AppComponent = JSON.parse(JSON.stringify(node.data))
				$dirtyStore = true

				deleteNodeById($app.grid, component.id)

				const newItem = createNewGridItem($app.grid, component.id, data)
				$app.grid = [...$app.grid, newItem]
			}
		}
	}
</script>

{#if component}
	<select id="move-to-grid" class="w-full">
		<option value="main:grid"> Main grid </option>

		{#each availableGrids as grid}
			<option value={grid.parentComponentId + ':' + grid.subGridIndex}>
				{grid.parentComponentId} - {grid.subGridIndex}
			</option>
		{/each}
	</select>
	<Button
		size="xs"
		color="dark"
		startIcon={{ icon: faCopy }}
		on:click={() => {
			if (component) {
				onMove()
			}
		}}
	>
		Move component: {component.id}
	</Button>
{/if}
