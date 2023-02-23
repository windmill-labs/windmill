<script lang="ts">
	import { Button } from '$lib/components/common'
	import { faCopy } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import type { AppEditorContext, GridItem } from '../../types'
	import { findParent } from '../../utils'
	import type { AppComponent } from '../component'

	export let component: AppComponent | undefined

	const { app } = getContext<AppEditorContext>('AppEditorContext')

	function listAllSubGrids(root: GridItem[]): GridItem[][] {
		const subGrids: GridItem[][] = []

		function findSubGrids(a: GridItem): void {
			if (a.data.subGrids) {
				for (const subGrid of a.data.subGrids) {
					subGrids.push(subGrid)
					for (const innerA of subGrid) {
						findSubGrids(innerA)
					}
				}
			}
		}

		for (const a of root) {
			findSubGrids(a)
		}

		return subGrids
	}

	function moveComponent(
		root: GridItem[],
		id: string,
		currentSubGridIndex: number,
		targetId: string,
		targetSubGridIndex: number
	): GridItem[] {
		const parent = findParent(root, id)

		if (!parent) {
			throw new Error(`Parent A object with ID ${id} not found.`)
		}

		const currentSubGrid = parent.data.subGrids[currentSubGridIndex]

		if (!currentSubGrid) {
			throw new Error(
				`Sub-grid with index ${currentSubGridIndex} not found for parent A object with ID ${id}.`
			)
		}

		const targetParentA = findParent(root, targetId)

		if (!targetParentA) {
			throw new Error(`Target parent A object with ID ${targetId} not found.`)
		}
		const targetSubGrid = targetParentA.data.subGrids[targetSubGridIndex]

		if (!targetSubGrid) {
			throw new Error(
				`Target sub-grid with index ${targetSubGridIndex} not found for parent A object with ID ${targetId}.`
			)
		}

		const index = currentSubGrid.findIndex((a) => a.id === id)

		if (index === -1) {
			throw new Error(`A object with ID ${id} not found in current sub-grid.`)
		}

		const removedA = currentSubGrid.splice(index, 1)[0]
		targetSubGrid.push(removedA)

		return root
	}
	$: x = listAllSubGrids($app.grid)
	$: console.log(x)
</script>

{#if component}
	<Button
		size="xs"
		color="dark"
		startIcon={{ icon: faCopy }}
		on:click={() => {
			if (component) {
			}
		}}
	>
		Move component: {component.id}
	</Button>
{/if}
