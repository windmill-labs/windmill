<script lang="ts">
	import { Button } from '$lib/components/common'
	import { dirtyStore } from '$lib/components/common/confirmationModal/dirtyStore'
	import { push } from '$lib/history'
	import { faCopy } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import type { App, AppEditorContext } from '../../types'
	import {
		createNewGridItem,
		deleteGridItem,
		findGridItem,
		getAllSubgridsAndComponentIds,
		insertNewGridItem
	} from '../appUtils'
	import type { AppComponent } from '../component'

	export let component: AppComponent | undefined
	export let parent: string | undefined

	let selectedOption: string

	const { app, history } = getContext<AppEditorContext>('AppEditorContext')

	function listAllSubGrids(app: App) {
		return app.subgrids ? Object.keys(app.subgrids) : []
	}

	function findAndDelete(id: string) {
		const node = findGridItem($app, id)

		if (!node) {
			return
		}

		const data: AppComponent = JSON.parse(JSON.stringify(node.data))
		const shouldKeepSubGrid = data.numberOfSubgrids ? data.numberOfSubgrids >= 1 : false

		$dirtyStore = true

		deleteGridItem($app, data, parent, shouldKeepSubGrid)

		return data
	}

	function insertComponentInSubGrid(
		data: AppComponent,
		targetId: string,
		targetSubGridIndex: number
	) {
		push(history, $app)
		const focusedGrid = {
			parentComponentId: targetId,
			subGridIndex: targetSubGridIndex
		}

		insertNewGridItem($app, data, focusedGrid, true)

		$app.grid = [...$app.grid]
	}

	function insertComponentInMainGrid(data: AppComponent) {
		const newComponent = createNewGridItem($app.grid, data.id, data)
		$app.grid = [...$app.grid, newComponent]
	}

	function onMove(component: AppComponent) {
		push(history, $app)
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

	$: [subgrids] = component ? getAllSubgridsAndComponentIds($app, component) : [[], []]
	$: availableGrids = listAllSubGrids($app)
	$: options = availableGrids
		? [
				defaultOption,
				...availableGrids?.map((grid) => ({
					label: grid,
					value: grid,
					disabled: grid === parent || subgrids.includes(grid)
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
