<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '../types'
	import Grid from 'svelte-grid'
	import ComponentEditor from './ComponentEditor.svelte'
	import { classNames } from '$lib/utils'

	const { selectedComponent, app, mode } = getContext<AppEditorContext>('AppEditorContext')

	const COLS = 6
	const cols: [number, number][] = [[1200, COLS]]

	$: if ($mode === 'preview') {
		$app.grid.map((c) => {
			c[COLS].customDragger = true
			c[COLS].customResizer = true
			return c
		})
	} else {
		$app.grid.map((c) => {
			c[COLS].customDragger = false
			c[COLS].customResizer = false
			return c
		})
	}
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<div class="bg-white h-full" on:click|preventDefault={() => $selectedComponent = undefined}>
	<Grid bind:items={$app.grid} rowHeight={32} let:dataItem {cols}>
		{#each $app.grid as gridComponent (gridComponent.id)}
			{#if gridComponent.data.id === dataItem.data.id}
				<!-- svelte-ignore a11y-click-events-have-key-events -->

				<div
					class={classNames(
						'h-full w-full flex justify-center align-center border border-gray-100'
					)}
					on:click|preventDefault|stopPropagation={() => {
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
