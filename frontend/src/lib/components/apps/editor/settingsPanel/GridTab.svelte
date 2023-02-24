<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { faPlus, faTrashAlt } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import type { AppEditorContext, GridItem } from '../../types'
	import { deleteComponent } from '../../utils'
	import PanelSection from './common/PanelSection.svelte'

	export let tabs: string[]
	export let subGrids: GridItem[][]

	const { runnableComponents, staticOutputs, app } =
		getContext<AppEditorContext>('AppEditorContext')

	function addTab() {
		tabs = [...tabs, `Tab ${tabs.length + 1}`]
		subGrids = [...subGrids, []]
	}

	function deleteSubgrid(index: number) {
		subGrids[index].forEach((x) => {
			deleteComponent(undefined, x.data, $app, $staticOutputs, $runnableComponents)
		})
		tabs.splice(index, 1)
		subGrids.splice(index, 1)
		tabs = tabs
		subGrids = subGrids
		$app = $app
	}
</script>

<PanelSection title={`Tabs ${tabs.length > 0 ? `(${tabs.length})` : ''}`}>
	<svelte:fragment slot="action">
		<Button
			size="xs"
			color="light"
			variant="border"
			startIcon={{ icon: faPlus }}
			on:click={addTab}
			iconOnly
		/>
	</svelte:fragment>

	{#if tabs.length == 0}
		<span class="text-xs text-gray-500">No Tabs</span>
	{/if}
	<div class="flex gap-2 flex-col mt-2">
		{#each tabs as value, index (index)}
			<div class="flex flex-row gap-2 items-center relative">
				<input type="text" bind:value />

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
	</div>
</PanelSection>
