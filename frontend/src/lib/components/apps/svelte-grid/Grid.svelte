<script lang="ts" context="module">
	import { writable } from 'svelte/store'

	const componentDraggedIdStore = writable<string | undefined>(undefined)
	const overlappedStore = writable<string | undefined>(undefined)
	const fakeShadowStore = writable<{
		x: number
		y: number
		xPerPx: number
		yPerPx: number
		w: number
		h: number
	}>({
		x: 0,
		y: 0,
		xPerPx: 0,
		yPerPx: 0,
		w: 0,
		h: 0
	})
</script>

<script lang="ts">
	import { twMerge } from 'tailwind-merge'

	import { getContainerHeight } from './utils/container'
	import { moveItem, getItemById, specifyUndefinedColumns } from './utils/item'
	import { onMount, createEventDispatcher } from 'svelte'
	import { getColumn, throttle } from './utils/other'
	import MoveResize from './MoveResize.svelte'
	import type { FilledItem } from './types'
	import {
		isContainer,
		ROW_GAP_X,
		ROW_GAP_Y,
		ROW_HEIGHT,
		sortGridItemsPosition
	} from '../editor/appUtils'

	const dispatch = createEventDispatcher()

	type T = $$Generic

	export let items: FilledItem<T>[]
	export let rowHeight: number = ROW_HEIGHT
	export let cols: [number, number][]
	export let gap = [ROW_GAP_X, ROW_GAP_Y]
	export let throttleUpdate = 100
	export let throttleResize = 100
	export let selectedIds: string[] | undefined
	export let allIdsInPath: string[] | undefined
	export let containerWidth: number | undefined = undefined
	export let scroller: HTMLElement | undefined = undefined
	export let sensor = 20
	export let root: boolean = false
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

	let isCtrlOrMetaPressed = false

	function handleKeyDown(event) {
		if (event.key === 'Control' || event.key === 'Meta') {
			isCtrlOrMetaPressed = true
		}
	}

	function handleKeyUp(event) {
		if (event.key === 'Control' || event.key === 'Meta') {
			isCtrlOrMetaPressed = false
		}
	}

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

				if (isCtrlOrMetaPressed) {
					const initialFixedStates = new Map()

					const fixedContainer = sortedItems.map((item) => {
						if (isContainer(item.data['type'])) {
							initialFixedStates.set(item, {
								item3Fixed: item[3].fixed,
								item12Fixed: item[12].fixed
							})

							item[3].fixed = true
							item[12].fixed = true
						}
						return item
					})

					moveItem(activeItem, fixedContainer, getComputedCols)

					// After the move, restore the initial fixed state using the map
					fixedContainer.forEach((item) => {
						if (initialFixedStates.has(item)) {
							const initialState = initialFixedStates.get(item)
							item[3].fixed = initialState.item3Fixed
							item[12].fixed = initialState.item12Fixed
						}
					})

					sortedItems = sortedItems
				} else {
					let { items } = moveItem(activeItem, sortedItems, getComputedCols)

					sortedItems = items
				}
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

	//let hiddenComponents = writable({})

	const handleRepaint = ({ detail }) => {
		if (!detail.isPointerUp) {
			throttleMatrix({ detail })
		} else {
			updateMatrix({ detail })
		}

		/**
		setTimeout(() => {
			$hiddenComponents = {
				...$hiddenComponents,
				[detail.id]: updateComponentVisibility(detail, sortedItems, getComputedCols)
			}
		}, 0)
		*/
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

		$fakeShadowStore = {
			x: detail.shadow.x,
			y: detail.shadow.y,
			xPerPx: detail.shadow.xPerPx,
			yPerPx: detail.shadow.yPerPx,
			w: detail.shadow.w,
			h: detail.shadow.h
		}

		if (!isCtrlOrMetaPressed) {
			$overlappedStore = undefined
			return
		}

		$overlappedStore = detail.intersectingElement
	}

	export function handleInitMove(id: string) {
		$componentDraggedIdStore = id

		Object.entries(moveResizes).forEach(([id, moveResize]) => {
			if (selectedIds?.includes(id)) {
				moveResize?.initmove()
			}
		})
	}
</script>

<svelte:window on:keydown={handleKeyDown} on:keyup={handleKeyUp} />

<div
	class="svlt-grid-container"
	style="height: {containerHeight}px"
	bind:this={container}
	id={root ? 'root-grid' : undefined}
	data-xperpx={xPerPx}
>
	{#if isCtrlOrMetaPressed && root}
		<!-- svelte-ignore a11y-no-static-element-interactions -->
		<!-- svelte-ignore a11y-mouse-events-have-key-events -->
		<div
			class={twMerge(
				'absolute inset-0  flex-col rounded-md bg-blue-100 dark:bg-gray-800 bg-opacity-50',
				'outline-dashed outline-offset-2 outline-2 outline-blue-300 dark:outline-blue-700',
				$componentDraggedIdStore && $overlappedStore === undefined
					? 'bg-draggedover dark:bg-draggedover-dark'
					: ''
			)}
		/>
		{#if $overlappedStore === undefined && $componentDraggedIdStore}
			<div class="absolute left-0 top-0 right-0">
				<div class="relative h-full w-full">
					<div
						class="absolute bg-blue-300 transition-all duration-75"
						style={`
								left: calc(${$fakeShadowStore.x * xPerPx + gapX}px + 0.5rem);
								top: calc(${$fakeShadowStore.y * yPerPx + gapY}px + 0.5rem);
								width: ${$fakeShadowStore.w * xPerPx}px;
								height: ${$fakeShadowStore.h * yPerPx}px;
							`}
					/>
				</div>
			</div>
		{/if}
	{/if}

	{#each sortedItems as item (item.id)}
		{#if item[getComputedCols] != undefined}
			{#if item.id === $overlappedStore && $componentDraggedIdStore}
				<div
					class="absolute"
					style={`
						left: ${item[getComputedCols].x * xPerPx + gapX}px;
						top: ${item[getComputedCols].y * yPerPx + gapY}px;
					`}
				>
					<div class="relative h-full w-full">
						<div
							class="absolute bg-blue-300 transition-all duration-75"
							style={`
								left: calc(${$fakeShadowStore.x * $fakeShadowStore.xPerPx + gapX}px + 0.5rem);
								top: calc(${$fakeShadowStore.y * $fakeShadowStore.yPerPx + gapY}px + 0.5rem);
								width: ${$fakeShadowStore.w * $fakeShadowStore.xPerPx}px;
								height: ${$fakeShadowStore.h * $fakeShadowStore.yPerPx}px;
							`}
						/>
					</div>
				</div>
			{/if}

			<MoveResize
				on:initmove={() => handleInitMove(item.id)}
				on:move={handleMove}
				bind:shadow={shadows[item.id]}
				bind:this={moveResizes[item.id]}
				on:repaint={handleRepaint}
				onTop={Boolean(allIdsInPath?.includes(item.id))}
				id={item.id}
				{xPerPx}
				{yPerPx}
				on:dropped={(e) => {
					$componentDraggedIdStore = undefined

					if (!isCtrlOrMetaPressed) {
						return
					}

					dispatch('dropped', {
						id: e.detail.id,
						overlapped: e.detail.overlapped,
						x: e.detail.x,
						y: e.detail.y
					})

					$overlappedStore = undefined
				}}
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
				overlapped={$overlappedStore}
				moveMode={isCtrlOrMetaPressed ? 'insert' : 'move'}
			>
				{#if item[getComputedCols]}
					<slot
						dataItem={item}
						hidden={false}
						overlapped={$overlappedStore}
						moveMode={isCtrlOrMetaPressed ? 'insert' : 'move'}
						componentDraggedId={$componentDraggedIdStore}
					/>
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
