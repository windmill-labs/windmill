<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { faPlus, faTrashAlt } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '../../types'
	import { deleteGridItem } from '../appUtils'
	import type { AppComponent } from '../component'
	import PanelSection from './common/PanelSection.svelte'

	export let panes: number[]
	export let component: AppComponent

	const { app, runnableComponents } = getContext<AppViewerContext>('AppViewerContext')

	function addTab() {
		const numberOfPanes = panes.length
		panes = Array(panes.length + 1)
			.fill(0)
			.map((_) => Math.floor(100 / (panes.length + 1)))

		if (!$app.subgrids) {
			$app.subgrids = {}
		}

		$app.subgrids[`${component.id}-${numberOfPanes}`] = []
		component.numberOfSubgrids = panes.length
	}

	function deleteSubgrid(index: number) {
		let subgrid = `${component.id}-${index}`
		for (const item of $app!.subgrids![subgrid]) {
			const components = deleteGridItem($app, item.data, subgrid, false)
			console.log(components)
			for (const key in components) {
				delete $runnableComponents[key]
			}
		}
		$runnableComponents = $runnableComponents
		for (let i = index; i < panes.length - 1; i++) {
			$app!.subgrids![`${component.id}-${i}`] = $app!.subgrids![`${component.id}-${i + 1}`]
		}
		panes.splice(index, 1)
		panes = panes
		component.numberOfSubgrids = panes.length
		$app = $app
	}
</script>

<PanelSection title={`panes ${panes.length > 0 ? `(${panes.length})` : ''}`}>
	{#if panes.length == 0}
		<span class="text-xs text-gray-500">No panes</span>
	{/if}
	<div class="w-full flex gap-2 flex-col mt-2">
		{#each panes as value, index (index)}
			<div class="w-full flex flex-row gap-2 items-center relative">
				<input on:keydown|stopPropagation type="number" bind:value />

				<div class="absolute top-1 right-1">
					<Button
						size="xs"
						color="light"
						variant="border"
						on:click={() => {
							deleteSubgrid(index)
						}}
						iconOnly
						btnClasses="!text-red-500"
						startIcon={{ icon: faTrashAlt }}
					/>
				</div>
			</div>
		{/each}
		<Button
			size="xs"
			color="light"
			variant="border"
			startIcon={{ icon: faPlus }}
			on:click={addTab}
			iconOnly
		/>
	</div>
</PanelSection>
