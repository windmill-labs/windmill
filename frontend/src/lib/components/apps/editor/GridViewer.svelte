<script lang="ts">
	import { onMount, createEventDispatcher } from 'svelte'

	import type { FilledItem } from '../svelte-grid/types'
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

	export let allIdsInPath: string[] | undefined = undefined
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
			{@const onTop = allIdsInPath?.includes(item.id)}
			{@const width =
				Math.min(getComputedCols, item[getComputedCols] && item[getComputedCols].w) * xPerPx -
				gapX * 2}
			{@const height = (item[getComputedCols] && item[getComputedCols].h) * yPerPx - gapY * 2}
			{@const top = (item[getComputedCols] && item[getComputedCols].y) * yPerPx + gapY}
			{@const left = (item[getComputedCols] && item[getComputedCols].x) * xPerPx + gapX}
			<div
				class="svlt-grid-item"
				style="width: {width}px; height:{height}px; {onTop
					? 'z-index: 1000;'
					: ''} top: {top}px; left: {left}px;"
			>
				{#if item[getComputedCols]}
					<slot dataItem={item} item={item[getComputedCols]} />
				{/if}
			</div>
		{/each}
	{/if}
</div>

<style>
	.svlt-grid-container {
		position: relative;
		width: 100%;
	}
	.svlt-grid-item {
		touch-action: none;
		position: absolute;
		will-change: auto;
		backface-visibility: hidden;
		-webkit-backface-visibility: hidden;
	}
</style>
