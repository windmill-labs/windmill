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

		function findSubGrids(a: GridItem): void {
			if (a.data.subGrids) {
				for (const subGrid of a.data.subGrids) {
					subGrids.push({
						parentComponentId: a.id,
						subGridIndex: a.data.subGrids.indexOf(subGrid)
					})
					for (const innerA of subGrid) {
						findSubGrids(innerA)
					}
				}
			}
		}

		for (const a of root) {
			findSubGrids(a)
		}

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
