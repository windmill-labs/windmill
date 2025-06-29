<script lang="ts">
	import { createBubbler, stopPropagation } from 'svelte/legacy'

	const bubble = createBubbler()
	import Button from '$lib/components/common/button/Button.svelte'
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '../../types'
	import { deleteGridItem } from '../appUtils'
	import type { AppComponent } from '../component'
	import PanelSection from './common/PanelSection.svelte'
	import { Plus, Trash } from 'lucide-svelte'

	interface Props {
		panes: number[]
		component: AppComponent
	}

	let { panes = $bindable(), component = $bindable() }: Props = $props()

	const { app, runnableComponents } = getContext<AppViewerContext>('AppViewerContext')

	function addTab() {
		const numberOfPanes = panes.length
		if (!$app.subgrids) {
			$app.subgrids = {}
		}
		$app.subgrids[`${component.id}-${numberOfPanes}`] = []
		component.numberOfSubgrids = panes.length + 1

		panes = Array(panes.length + 1)
			.fill(0)
			.map((_) => Math.floor(100 / (panes.length + 1)))
	}

	function deleteSubgrid(index: number) {
		let subgrid = `${component.id}-${index}`
		for (const item of $app!.subgrids![subgrid]) {
			const components = deleteGridItem($app, item.data, subgrid)
			for (const key in components) {
				delete $runnableComponents[key]
			}
		}
		$runnableComponents = $runnableComponents
		for (let i = index; i < panes.length - 1; i++) {
			$app!.subgrids![`${component.id}-${i}`] = $app!.subgrids![`${component.id}-${i + 1}`]
		}
		panes.splice(index, 1)
		delete $app!.subgrids![`${component.id}-${panes.length}`]

		panes = panes
		component.numberOfSubgrids = panes.length
		$app = $app
	}
</script>

<PanelSection title={`panes ${panes.length > 0 ? `(${panes.length})` : ''}`}>
	{#if panes.length == 0}
		<span class="text-xs text-tertiary">No panes</span>
	{/if}
	<div class="w-full flex gap-2 flex-col">
		{#each panes as _, index (index)}
			<div class="w-full flex flex-row gap-1 items-center relative">
				<input
					onkeydown={stopPropagation(bubble('keydown'))}
					type="number"
					bind:value={panes[index]}
				/>

				<Button
					size="xs"
					color="light"
					on:click={() => {
						deleteSubgrid(index)
					}}
					iconOnly
					startIcon={{ icon: Trash }}
				/>
			</div>
		{/each}
		<Button
			size="xs"
			color="light"
			variant="border"
			startIcon={{ icon: Plus }}
			on:click={addTab}
			iconOnly
		/>
	</div>
</PanelSection>
