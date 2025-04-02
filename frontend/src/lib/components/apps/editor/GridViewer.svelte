<script lang="ts">
	import { columnConfiguration, WIDE_GRID_COLUMNS } from '../gridUtils'

	import { ROW_GAP_X, ROW_GAP_Y, ROW_HEIGHT } from './appUtils'

	import type { AppViewerContext, EditorBreakpoint } from '../types'

	import { onMount, createEventDispatcher, getContext } from 'svelte'

	import type { FilledItem } from '../svelte-grid/types'
	import { getColumn, throttle } from '../svelte-grid/utils/other'
	import { getContainerHeight } from '../svelte-grid/utils/container'
	import { specifyUndefinedColumns } from '../svelte-grid/utils/item'

	const dispatch = createEventDispatcher()

	type T = $$Generic

	const { app } = getContext<AppViewerContext>('AppViewerContext')

	export let items: FilledItem<T>[]
	export let rowHeight: number = ROW_HEIGHT
	export let gap = [ROW_GAP_X, ROW_GAP_Y]
	export let throttleUpdate = 100
	export let maxRow: number
	export let breakpoint: EditorBreakpoint

	export let allIdsInPath: string[] | undefined = undefined
	export let containerWidth: number | undefined = undefined

	export let parentWidth: number | undefined = undefined

	const cols = columnConfiguration

	let showSkeleton = false

	let getComputedCols: 3 | 12 | undefined =
		$app.mobileViewOnSmallerScreens == false ? WIDE_GRID_COLUMNS : undefined

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
		setTimeout(() => {
			showSkeleton = true
		}, 100)
		const sizeObserver = new ResizeObserver((entries) => {
			requestAnimationFrame(() => {
				let width = entries[0].contentRect.width
				if (width === 0) {
					width = 1
				}
				if (width === containerWidth) return
				if ($app.mobileViewOnSmallerScreens != false || !getComputedCols) {
					getComputedCols = getColumn(parentWidth ?? width, cols)
				}

				xPerPx = width / getComputedCols!

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
	{#if xPerPx && getComputedCols}
		{#each items as item (item.id)}
			{@const onTop = allIdsInPath?.includes(item.id)}
			{@const width =
				Math.min(getComputedCols, item[getComputedCols] && item[getComputedCols].w) * xPerPx -
				gapX * 2}
			{@const height =
				(item?.[breakpoint === 'sm' ? 3 : 12]?.fullHeight
					? maxRow - item[getComputedCols].y
					: item[getComputedCols] && item[getComputedCols].h) *
					yPerPx -
				gapY * 2}
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
	{:else if showSkeleton}
		<div
			class="h-full w-full flex-col animate-skeleton dark:bg-frost-900/50 [animation-delay:1000ms]"
		></div>
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
