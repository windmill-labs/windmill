<script lang="ts">
	import { Button } from '$lib/components/common'
	import { faCopy } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import type { AppEditorContext, FocusedGrid, GridItem } from '../../types'
	import type { AppComponent } from '../component'

	export let component: AppComponent | undefined

	const { app, focusedGrid } = getContext<AppEditorContext>('AppEditorContext')

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

	function moveComponent(
		root: GridItem[],
		id: string,
		currentSubGridIndex: number,
		targetId: string,
		targetSubGridIndex: number
	): GridItem[] {
		return root
	}

	$: availableGrids = listAllSubGrids($app.grid)

	function onMove() {
		if (component) {
			const [targetId, targetSubGridIndex] = (
				document.getElementById('move-to-grid') as HTMLSelectElement
			).value.split(':')

			if ($focusedGrid) {
				$app.grid = moveComponent(
					$app.grid,
					component.id,
					$focusedGrid?.subGridIndex,
					targetId,
					parseInt(targetSubGridIndex)
				)
			}
		}
	}
</script>

{#if component}
	<select id="move-to-grid" class="w-full">
		<option value="main"> Main grid </option>
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
