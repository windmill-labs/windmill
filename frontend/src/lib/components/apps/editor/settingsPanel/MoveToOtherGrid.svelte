<script lang="ts">
	import { Button } from '$lib/components/common'
	import { dirtyStore } from '$lib/components/common/confirmationModal/dirtyStore'
	import { faCopy } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import type { App, AppEditorContext } from '../../types'
	import { createNewGridItem, deleteGridItem, findGridItem, insertNewGridItem } from '../appUtils'
	import type { AppComponent } from '../component'

	export let component: AppComponent | undefined
	export let parent: string | undefined

	const { app, selectedComponent } = getContext<AppEditorContext>('AppEditorContext')

	function listAllSubGrids(app: App) {
		return app.subgrids ? Object.keys(app.subgrids) : []
	}

	function moveComponent(id: string, targetId: string, targetSubGridIndex: number) {
		const node = findGridItem($app, id)

		if (!node) {
			return
		}

		const data: AppComponent = JSON.parse(JSON.stringify(node.data))
		$dirtyStore = true

		deleteGridItem($app, data, parent)

		const newId = insertNewGridItem($app, data, {
			parentComponentId: targetId,
			subGridIndex: targetSubGridIndex
		})

		$selectedComponent = newId
	}

	$: availableGrids = listAllSubGrids($app)

	function onMove() {
		if (component) {
			const [targetId, targetSubGridIndex] = (
				document.getElementById('move-to-grid') as HTMLSelectElement
			).value.split('-')

			if (targetId !== 'main' && targetSubGridIndex !== 'grid') {
				moveComponent(component.id, targetId, parseInt(targetSubGridIndex))
			} else {
				const node = findGridItem($app, component.id)

				if (!node) {
					return
				}

				const data: AppComponent = JSON.parse(JSON.stringify(node.data))
				$dirtyStore = true

				deleteGridItem($app, component, parent)

				const newComponent = createNewGridItem($app.grid, component.id, data)
				$app.grid = [...$app.grid, newComponent]
				$selectedComponent = newComponent.id
			}
		}
	}
</script>

{#if component}
	<select id="move-to-grid" class="w-full">
		<option value="main-grid"> Main grid </option>

		{#each availableGrids as grid}
			<option value={grid}>
				{grid}
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
