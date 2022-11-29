<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '../types'
	import Grid from 'svelte-grid'
	import ComponentEditor from './ComponentEditor.svelte'
	import { classNames } from '$lib/utils'
	import { columnConfiguration, disableDrag, enableDrag } from '../gridUtils'

	const { selectedComponent, app, mode } = getContext<AppEditorContext>('AppEditorContext')

	$: if ($mode === 'preview') {
		$app.grid.map((gridItem) => disableDrag(gridItem))
	} else {
		$app.grid.map((gridItem) => enableDrag(gridItem))
	}
</script>

<div class="bg-white h-full">
	<Grid bind:items={$app.grid} rowHeight={64} let:dataItem cols={columnConfiguration}>
		{#each $app.grid as gridComponent (gridComponent.id)}
			{#if gridComponent.data.id === dataItem.data.id}
				<!-- svelte-ignore a11y-click-events-have-key-events -->

				<div
					class={classNames(
						'h-full w-full flex justify-center align-center',
						gridComponent.data.card ? 'border border-gray-100' : ''
					)}
					on:click={() => {
						$selectedComponent = dataItem.data.id
					}}
				>
					<ComponentEditor
						bind:component={gridComponent.data}
						selected={$selectedComponent === dataItem.data.id}
					/>
				</div>
			{/if}
		{/each}
	</Grid>
</div>

<style>
	:global(.svlt-grid-shadow) {
		/* Back shadow */
		background: rgb(147 197 253) !important;
	}
</style>
