<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '../types'
	import Grid from 'svelte-grid'
	import ComponentEditor from './ComponentEditor.svelte'

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

<div class="bg-white">
	<Grid bind:items={$app.grid} rowHeight={64} let:dataItem {cols}>
		{@const index = $app.grid.findIndex((c) => c.data.id === dataItem.data.id)}
		<!-- svelte-ignore a11y-click-events-have-key-events -->
		<div
			class="h-full w-full flex justify-center align-center border border-gray-100"
			on:click={() => {
				$selectedComponent = dataItem.data.id
			}}
		>
			<ComponentEditor
				bind:component={$app.grid[index].data}
				selected={$selectedComponent === dataItem.data.id}
			/>
		</div>
	</Grid>
</div>

<style>
	:global(.svlt-grid-shadow) {
		/* Back shadow */
		background: lightblue !important;
	}
</style>
