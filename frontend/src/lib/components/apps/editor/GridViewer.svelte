<script lang="ts">
	import { onMount, createEventDispatcher } from 'svelte'

	import type { FilledItem } from '../svelte-grid/types'
	import GridViewerComponent from './GridViewerComponent.svelte'
	import { getColumn, throttle } from '../svelte-grid/utils/other'
	import { getContainerHeight } from '../svelte-grid/utils/container'
	import { specifyUndefinedColumns } from '../svelte-grid/utils/item'

	const dispatch = createEventDispatcher()

	type T = $$Generic

	export let items: FilledItem<T>[]
	export let rowHeight: number
	export let cols: [number, number][]
	export let gap = [10, 10]
	export let throttleUpdate = 100

	export let onTopId: string | undefined = undefined
	export let containerWidth: number | undefined = undefined

	export let parentWidth: number | undefined = undefined

	let getComputedCols

	let container

	$: [gapX, gapY] = gap

	let xPerPx = 0
	let yPerPx = rowHeight

	$: containerHeight = getContainerHeight(items, yPerPx, getComputedCols)

	const onResize = throttle(() => {
		items = specifyUndefinedColumns(items, getComputedCols, cols)
		dispatch('resize', {
			cols: getComputedCols,
			xPerPx,
			yPerPx,
			width: containerWidth
		})
	}, throttleUpdate)

	onMount(() => {
		const sizeObserver = new ResizeObserver((entries) => {
			requestAnimationFrame(() => {
				let width = entries[0].contentRect.width

				if (width === containerWidth) return

				getComputedCols = getColumn(parentWidth ?? width, cols)

				xPerPx = width / getComputedCols

				if (!containerWidth) {
					items = specifyUndefinedColumns(items, getComputedCols, cols)

					dispatch('mount', {
						cols: getComputedCols,
						xPerPx,
						yPerPx // same as rowHeight
					})
				} else {
					onResize()
				}

				containerWidth = width
			})
		})

		sizeObserver.observe(container)

		return () => sizeObserver.disconnect()
	})
</script>

<div class="svlt-grid-container" style="height: {containerHeight}px" bind:this={container}>
	{#if xPerPx}
		{#each items as item (item.id)}
			<GridViewerComponent
				onTop={item.id == onTopId}
				width={Math.min(getComputedCols, item[getComputedCols] && item[getComputedCols].w) *
					xPerPx -
					gapX * 2}
				height={(item[getComputedCols] && item[getComputedCols].h) * yPerPx - gapY * 2}
				top={(item[getComputedCols] && item[getComputedCols].y) * yPerPx + gapY}
				left={(item[getComputedCols] && item[getComputedCols].x) * xPerPx + gapX}
			>
				{#if item[getComputedCols]}
					<slot dataItem={item} item={item[getComputedCols]} />
				{/if}
			</GridViewerComponent>
		{/each}
	{/if}
</div>

<style>
	.svlt-grid-container {
		position: relative;
		width: 100%;
	}
</style>
