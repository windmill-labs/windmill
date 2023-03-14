<script lang="ts">
	import { getContainerHeight } from './utils/container'
	import { moveItemsAroundItem, moveItem, getItemById, specifyUndefinedColumns } from './utils/item'
	import { onMount, createEventDispatcher } from 'svelte'
	import { getColumn, throttle } from './utils/other'
	import MoveResize from './MoveResize.svelte'
	import type { FilledItem } from './types'
	import type { Writable } from 'svelte/store'

	const dispatch = createEventDispatcher()

	type T = $$Generic

	export let fillSpace = false
	export let items: FilledItem<T>[]
	export let rowHeight: number
	export let cols: [number, number][]
	export let gap = [10, 10]
	export let fastStart = false
	export let throttleUpdate = 100
	export let throttleResize = 100
	export let onTopId: string | undefined = undefined
	export let containerWidth: number | undefined = undefined

	export let scroller = undefined
	export let sensor = 20

	export let parentWidth: number | undefined = undefined

	let getComputedCols

	let container

	$: [gapX, gapY] = gap

	let xPerPx = 0
	let yPerPx = rowHeight

	$: containerHeight = getContainerHeight(items, yPerPx, getComputedCols)

	const pointerup = (ev) => {
		dispatch('pointerup', {
			id: ev.detail.id,
			cols: getComputedCols
		})
	}

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

	let initItems: FilledItem<T>[] | undefined = undefined
	const updateMatrix = ({ detail }) => {
		let isPointerUp = detail.isPointerUp
		let citems: FilledItem<T>[]
		if (isPointerUp) {
			try {
				citems = JSON.parse(JSON.stringify(initItems))
			} catch (e) {
				citems = JSON.parse(JSON.stringify(items))
			}
			initItems = undefined
		} else {
			if (initItems == undefined) {
				initItems = JSON.parse(JSON.stringify(items))
			}
			citems = JSON.parse(JSON.stringify(initItems))
		}

		let activeItem = getItemById(detail.id, citems)

		if (activeItem) {
			activeItem = {
				...activeItem,
				[getComputedCols]: {
					...activeItem[getComputedCols],
					...detail.shadow
				}
			}

			if (fillSpace) {
				items = moveItemsAroundItem(
					activeItem,
					citems,
					getComputedCols,
					getItemById(detail.id, citems)
				)
			} else {
				items = moveItem(activeItem, citems, getComputedCols, getItemById(detail.id, citems))
			}

			if (detail.onUpdate) detail.onUpdate()

			dispatch('change', {
				unsafeItem: activeItem,
				id: activeItem.id,
				cols: getComputedCols
			})
		}

		if (isPointerUp) {
			dispatch('redraw', items)
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
</script>

<div class="svlt-grid-container" style="height: {containerHeight}px" bind:this={container}>
	{#if xPerPx || !fastStart}
		{#each items as item, i (item.id)}
			<MoveResize
				on:repaint={handleRepaint}
				on:pointerup={pointerup}
				onTop={item.id == onTopId}
				id={item.id}
				resizable={item[getComputedCols] && item[getComputedCols].resizable}
				draggable={item[getComputedCols] && item[getComputedCols].draggable}
				{xPerPx}
				{yPerPx}
				width={Math.min(getComputedCols, item[getComputedCols] && item[getComputedCols].w) *
					xPerPx -
					gapX * 2}
				height={(item[getComputedCols] && item[getComputedCols].h) * yPerPx - gapY * 2}
				top={(item[getComputedCols] && item[getComputedCols].y) * yPerPx + gapY}
				left={(item[getComputedCols] && item[getComputedCols].x) * xPerPx + gapX}
				item={item[getComputedCols]}
				min={item[getComputedCols] && item[getComputedCols].min}
				max={item[getComputedCols] && item[getComputedCols].max}
				cols={getComputedCols}
				{gapX}
				{gapY}
				{sensor}
				container={scroller}
				nativeContainer={container}
				let:resizePointerDown
				let:movePointerDown
			>
				{#if item[getComputedCols]}
					<slot
						{movePointerDown}
						{resizePointerDown}
						dataItem={item}
						item={item[getComputedCols]}
						index={i}
					/>
				{/if}
			</MoveResize>
		{/each}
	{/if}
</div>

<style>
	.svlt-grid-container {
		position: relative;
		width: 100%;
	}
</style>
