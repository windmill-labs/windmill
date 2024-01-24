<script lang="ts">
	import { getContainerHeight } from './utils/container'
	import { moveItem, getItemById, specifyUndefinedColumns } from './utils/item'
	import { onMount, createEventDispatcher } from 'svelte'
	import { getColumn, throttle } from './utils/other'
	import MoveResize from './MoveResize.svelte'
	import type { FilledItem } from './types'
	import { sortGridItemsPosition } from '../editor/appUtils'

	const dispatch = createEventDispatcher()

	type T = $$Generic

	export let items: FilledItem<T>[]
	export let rowHeight: number
	export let cols: [number, number][]
	export let gap = [10, 10]
	export let throttleUpdate = 100
	export let throttleResize = 100
	export let selectedIds: string[] | undefined
	export let allIdsInPath: string[] | undefined
	export let containerWidth: number | undefined = undefined

	export let scroller: HTMLElement | undefined = undefined
	export let sensor = 20

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

	let sortedItems: FilledItem<T>[] = []
	$: sortedItems = JSON.parse(JSON.stringify(items)).sort((a, b) => a.id.localeCompare(b.id))

	let initItems: FilledItem<T>[] | undefined = undefined
	const updateMatrix = ({ detail }) => {
		let isPointerUp = detail.isPointerUp
		let citems: FilledItem<T>[]
		if (isPointerUp) {
			try {
				citems = JSON.parse(JSON.stringify(initItems))
			} catch (e) {
				citems = JSON.parse(JSON.stringify(sortedItems))
			}
			initItems = undefined
		} else {
			if (initItems == undefined) {
				initItems = JSON.parse(JSON.stringify(sortedItems))
			}
			citems = JSON.parse(JSON.stringify(initItems))
		}
		let nselectedIds = selectedIds ?? []
		if (detail.id && !selectedIds?.includes(detail.id)) {
			nselectedIds = [detail.id, ...(selectedIds ?? [])]
		}
		sortedItems = citems
		for (let id of nselectedIds) {
			let activeItem = getItemById(id, sortedItems)

			if (activeItem) {
				activeItem = {
					...activeItem,
					[getComputedCols]: {
						...activeItem[getComputedCols],
						...shadows[id]
					}
				}

				sortedItems = moveItem(activeItem, sortedItems, getComputedCols)
			}
		}

		for (let id of nselectedIds ?? []) {
			if (detail.activate) {
				moveResizes?.[id]?.inActivate()
			}
		}

		if (isPointerUp) {
			dispatch(
				'redraw',
				sortGridItemsPosition(JSON.parse(JSON.stringify(sortedItems)), getComputedCols)
			)
		}
	}

	const throttleMatrix = throttle(updateMatrix, throttleResize)

	const handleRepaint = ({ detail }) => {
		if (!detail.isPointerUp) {
			throttleMatrix({ detail })
		} else {
			updateMatrix({ detail })
		}
	}
	let moveResizes: Record<string, MoveResize> = {}
	let shadows: Record<string, { x: number; y: number; w: number; h: number } | undefined> = {}

	export function handleMove({ detail }) {
		Object.entries(moveResizes).forEach(([id, moveResize]) => {
			if (selectedIds?.includes(id)) {
				moveResize?.updateMove(JSON.parse(JSON.stringify(detail.cordDiff)), detail.eventY)
			}
		})
		throttleMatrix({ detail: { isPointerUp: false, activate: false } })
	}

	export function handleInitMove({ detail }) {
		Object.entries(moveResizes).forEach(([id, moveResize]) => {
			if (selectedIds?.includes(id)) {
				moveResize?.initmove()
			}
		})
	}
</script>

<div class="svlt-grid-container" style="height: {containerHeight}px" bind:this={container}>
	{#each sortedItems as item (item.id)}
		{#if item[getComputedCols] != undefined}
			<MoveResize
				on:initmove={handleInitMove}
				on:move={handleMove}
				bind:shadow={shadows[item.id]}
				bind:this={moveResizes[item.id]}
				on:repaint={handleRepaint}
				onTop={Boolean(allIdsInPath?.includes(item.id))}
				id={item.id}
				{xPerPx}
				{yPerPx}
				width={xPerPx == 0
					? 0
					: Math.min(getComputedCols, item[getComputedCols] && item[getComputedCols].w) * xPerPx -
					  gapX * 2}
				height={(item[getComputedCols] && item[getComputedCols].h) * yPerPx - gapY * 2}
				top={(item[getComputedCols] && item[getComputedCols].y) * yPerPx + gapY}
				left={(item[getComputedCols] && item[getComputedCols].x) * xPerPx + gapX}
				item={item[getComputedCols]}
				cols={getComputedCols}
				{gapX}
				{gapY}
				{sensor}
				container={scroller}
				nativeContainer={container}
			>
				{#if item[getComputedCols]}
					<slot dataItem={item} />
				{/if}
			</MoveResize>
		{/if}
	{/each}
</div>

<style>
	.svlt-grid-container {
		position: relative;
		width: 100%;
		user-select: none;
	}
</style>
