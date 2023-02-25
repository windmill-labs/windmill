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
	let selectedOption: string

	const { app } = getContext<AppEditorContext>('AppEditorContext')

	function listAllSubGrids(app: App) {
		return app.subgrids ? Object.keys(app.subgrids) : []
	}

	function findAndDelete(id: string) {
		const node = findGridItem($app, id)

		if (!node) {
			return
		}

		const data: AppComponent = JSON.parse(JSON.stringify(node.data))
		$dirtyStore = true

		deleteGridItem($app, data, parent)
		return data
	}

	function insertComponentInSubGrid(
		data: AppComponent,
		targetId: string,
		targetSubGridIndex: number
	) {
		insertNewGridItem(
			$app,
			data,
			{
				parentComponentId: targetId,
				subGridIndex: targetSubGridIndex
			},
			true
		)
		$app.grid = [...$app.grid]
	}

	function insertComponentInMainGrid(data: AppComponent) {
		const newComponent = createNewGridItem($app.grid, data.id, data)
		$app.grid = [...$app.grid, newComponent]
	}

	function onMove(component: AppComponent) {
		const data = findAndDelete(component.id)

		if (!data) {
			return
		}

		if (selectedOption !== 'main-grid') {
			const [targetId, targetSubGridIndex] = selectedOption.split('-')
			insertComponentInSubGrid(data, targetId, parseInt(targetSubGridIndex))
		} else {
			insertComponentInMainGrid(data)
		}
	}

	const defaultOption = {
		label: 'Main grid',
		value: 'main-grid',
		disabled: parent === undefined
	}
	$: availableGrids = listAllSubGrids($app)
	$: options = availableGrids
		? [
				defaultOption,
				...availableGrids?.map((grid) => ({
					label: grid,
					value: grid,
					disabled: grid === parent || (component && grid.startsWith(component.id))
				}))
		  ]
		: [defaultOption]
</script>

{#if component && !options.every((option) => option.disabled)}
	<select bind:value={selectedOption} class="w-full">
		{#each options as option}
			<option value={option.value} disabled={option.disabled}>{option.label}</option>
		{/each}
	</select>
	<Button
		size="xs"
		color="dark"
		startIcon={{ icon: faCopy }}
		on:click={() => {
			if (component) {
				onMove(component)
			}
		}}
	>
		Move component: {component.id}
	</Button>
{:else}
	<p class="text-gray-500 text-sm">No grids available</p>
{/if}
