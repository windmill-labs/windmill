<script lang="ts" module>
	import { writable } from 'svelte/store'

	const componentDraggedIdStore = writable<string | undefined>(undefined)
	const componentDraggedParentIdStore = writable<string | undefined>(undefined)
	const overlappedStore = writable<string | undefined>(undefined)
	const fakeShadowStore = writable<GridShadow | undefined>(undefined)
</script>

<script lang="ts">
	import { columnConfiguration, moveMode, WIDE_GRID_COLUMNS } from '../gridUtils'

	import gridHelp from './utils/helper'
	import type { AppViewerContext, GridItem } from '../types'
	import { twMerge } from 'tailwind-merge'

	import { getContainerHeight } from './utils/container'
	import { moveItem, getItemById, specifyUndefinedColumns } from './utils/item'
	import { onMount, createEventDispatcher, getContext } from 'svelte'
	import { getColumn, throttle } from './utils/other'
	import MoveResize from './MoveResize.svelte'
	import type { FilledItem } from './types'
	import {
		areShadowsTheSame,
		findGridItemParentGrid,
		getDeltaXByComponent,
		getDeltaYByComponent,
		isContainer,
		ROW_GAP_X,
		ROW_GAP_Y,
		ROW_HEIGHT,
		sortGridItemsPosition,
		subGridIndexKey,
		type GridShadow
	} from '../editor/appUtils'

	const dispatch = createEventDispatcher()

	type T = $$Generic

	const { app, worldStore } = getContext<AppViewerContext>('AppViewerContext')

	interface Props {
		items: FilledItem<T>[]
		rowHeight?: number
		gap?: any
		throttleUpdate?: number
		throttleResize?: number
		selectedIds: string[] | undefined
		allIdsInPath: string[] | undefined
		containerWidth?: number | undefined
		scroller?: HTMLElement | undefined
		sensor?: number
		root?: boolean
		parentWidth?: number | undefined
		disableMove?: boolean
		children?: import('svelte').Snippet<[any]>
	}

	let {
		items,
		rowHeight = ROW_HEIGHT,
		gap = [ROW_GAP_X, ROW_GAP_Y],
		throttleUpdate = 100,
		throttleResize = 100,
		selectedIds,
		allIdsInPath,
		containerWidth = $bindable(undefined),
		scroller = undefined,
		sensor = 20,
		root = false,
		parentWidth = undefined,
		disableMove = false,
		children
	}: Props = $props()
	const cols = columnConfiguration

	let getComputedCols: 3 | 12 | undefined = $state(
		$app.mobileViewOnSmallerScreens == false ? WIDE_GRID_COLUMNS : undefined
	)
	let container = $state()

	let xPerPx = $state(0)
	let yPerPx = rowHeight

	const onResize = throttle(() => {
		if (!getComputedCols) return
		sortedItems = specifyUndefinedColumns(sortedItems, getComputedCols, cols)
		dispatch('resize', {
			cols: getComputedCols,
			xPerPx,
			yPerPx,
			width: containerWidth
		})
	}, throttleUpdate)

	let mounted = $state(false)

	onMount(() => {
		const sizeObserver = new ResizeObserver((entries) => {
			requestAnimationFrame(() => {
				let width = entries[0].contentRect.width
				if (width === 0) {
					width = 1
				}
				if (width === containerWidth) {
					mounted = true
					return
				}

				if ($app.mobileViewOnSmallerScreens != false || !getComputedCols) {
					getComputedCols = getColumn(parentWidth ?? width, cols)
				}
				xPerPx = width / getComputedCols!

				if (!containerWidth && getComputedCols) {
					sortedItems = specifyUndefinedColumns(sortedItems, getComputedCols, cols)

					dispatch('mount', {
						cols: getComputedCols,
						xPerPx,
						yPerPx // same as rowHeight
					})
				} else {
					onResize()
				}

				containerWidth = width
				mounted = true
			})
		})

		sizeObserver.observe(container as Element)

		return () => sizeObserver.disconnect()
	})

	let sortedItems: FilledItem<T>[] = $state([])

	let resizing: boolean = $state(false)

	function handleKeyUp(event) {
		if ((event.key === 'Control' || event.key === 'Meta') && root && $moveMode === 'insert') {
			setTimeout(() => {
				$moveMode = 'move'

				$fakeShadowStore = undefined
			}, 50)
		}
	}
	const initialFixedStates = new Map()

	let initItems: FilledItem<T>[] | undefined = undefined

	function smartCopy(items: FilledItem<T>[]) {
		return getComputedCols != undefined
			? items.map((item) => {
					return {
						...item,
						[getComputedCols as number]: { ...item[getComputedCols as number] }
					}
				})
			: []
	}
	const updateMatrix = ({ detail }) => {
		let isPointerUp = detail.isPointerUp
		let citems: FilledItem<T>[]
		if (isPointerUp) {
			if (initItems == undefined) {
				citems = smartCopy(sortedItems)
			} else {
				citems = smartCopy(initItems)
			}
			initItems = undefined
		} else {
			if (initItems == undefined) {
				initItems = smartCopy(sortedItems)
			}
			citems = smartCopy(initItems)
		}
		let nselectedIds = selectedIds ?? []
		if (detail.id && !selectedIds?.includes(detail.id)) {
			nselectedIds = [detail.id, ...(selectedIds ?? [])]
		}
		for (let id of nselectedIds) {
			let activeItem = getItemById(id, citems)

			if (activeItem && getComputedCols) {
				activeItem = {
					...activeItem,
					[getComputedCols]: {
						...activeItem[getComputedCols],
						...shadows[id]
					}
				}

				if ($moveMode === 'insert') {
					if ($componentDraggedParentIdStore === $overlappedStore) {
						const fixedContainer = citems.map((item) => {
							if (isContainer(item.data['type'])) {
								initialFixedStates.set(item.id, {
									item3Fixed: item[3].fixed,
									item12Fixed: item[12].fixed
								})

								item[3].fixed = true
								item[12].fixed = true
							}

							return item
						})

						let { items: nitems } = moveItem(activeItem, fixedContainer, getComputedCols)

						nitems = nitems.map((item) => {
							if (initialFixedStates.has(item.id)) {
								const initialState = initialFixedStates.get(item.id)

								if (initialState) {
									item[3].fixed = initialState.item3Fixed
									item[12].fixed = initialState.item12Fixed
								}
							}
							return item
						})

						sortedItems = nitems
					}
				} else {
					let { items: nitems } = moveItem(activeItem, citems, getComputedCols)

					sortedItems = nitems
				}
			}
		}

		for (let id of nselectedIds ?? []) {
			if (detail.activate) {
				moveResizes?.[id]?.inActivate()
			}
		}

		if (isPointerUp && getComputedCols) {
			dispatch('redraw', sortGridItemsPosition(smartCopy(sortedItems), getComputedCols))
		}
	}

	const throttleMatrix = throttle(updateMatrix, throttleResize)

	//let hiddenComponents = writable({})

	let lastDetail:
		| {
				cordDiff: { x: number; y: number }
				clientY: number
				intersectingElement?: string | undefined
				shadow?: GridShadow | undefined
				overlapped?: string | undefined
		  }
		| undefined = $state(undefined)

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

	function handleKeyDown(event) {
		if ((event.key === 'Control' || event.key === 'Meta') && $moveMode === 'move' && root) {
			if (resizing) {
				return
			}

			$moveMode = 'insert'
			if (lastDetail) {
				throttleMatrix({ detail: lastDetail })
				lastDetail = undefined
			}
		}
	}

	let moveResizes: Record<string, MoveResize> = {}
	let shadows: Record<string, { x: number; y: number; w: number; h: number } | undefined> = $state(
		{}
	)

	export function handleMove(detail: {
		cordDiff: { x: number; y: number }
		clientY: number
		intersectingElement?: string | undefined
		shadow?: GridShadow | undefined
		overlapped?: string | undefined
	}) {
		Object.entries(moveResizes).forEach(([id, moveResize]) => {
			if (selectedIds?.includes(id)) {
				moveResize?.updateMove(detail.cordDiff, detail.clientY)
			}
		})

		lastDetail = detail
		throttleMatrix({ detail: { isPointerUp: false, activate: false } })

		if ($moveMode === 'move') {
			$overlappedStore = undefined
			return
		}

		if (
			// We don't display the fake shadow if the dragged component is a child of the overlapped component
			$componentDraggedParentIdStore !== $overlappedStore &&
			detail.shadow &&
			// only update the fake shadow if the are different
			!areShadowsTheSame($fakeShadowStore, detail.shadow)
		) {
			const draggedItem = sortedItems.find((item) => item.id === $componentDraggedIdStore)

			if (draggedItem && getComputedCols) {
				draggedItem[getComputedCols].x = detail.shadow.x
				draggedItem[getComputedCols].y = detail.shadow.y
			}

			let nitems: GridItem[] = []

			if ($overlappedStore) {
				const div = document.getElementById(`component-${$overlappedStore}`)
				const type = div?.getAttribute('data-componenttype')

				if (!$app.subgrids) {
					return
				}

				const index = type ? subGridIndexKey(type, $overlappedStore, $worldStore) : 0

				nitems = $app.subgrids[`${$overlappedStore}-${index}`] ?? []
			} else {
				nitems = $app.grid ?? []
			}

			if (!draggedItem) {
				return
			}

			const freeSpace = gridHelp.findSpace(draggedItem, nitems, getComputedCols)

			$fakeShadowStore = {
				x: freeSpace.x,
				y: freeSpace.y,
				xPerPx: detail.shadow.xPerPx,
				yPerPx: detail.shadow.yPerPx,
				w: detail.shadow.w,
				h: detail.shadow.h
			}
		}

		// When leaving the overlapped component, we clear the fake shadow
		// to avoid rendering it with the wrong position at the next intersection
		if (detail.intersectingElement !== $overlappedStore) {
			$fakeShadowStore = undefined
		}

		// Update the overlapped component
		$overlappedStore = detail.intersectingElement
	}

	export function handleInitMove(id: string) {
		$componentDraggedIdStore = id
		$componentDraggedParentIdStore = findGridItemParentGrid($app, id)?.split('-')[0] ?? undefined

		Object.entries(moveResizes).forEach(([id, moveResize]) => {
			if (selectedIds?.includes(id)) {
				moveResize?.initmove()
			}
		})
	}
	let [gapX, gapY] = $derived(gap)
	let containerHeight = $derived(getContainerHeight(items, yPerPx, getComputedCols))
	$effect.pre(() => {
		sortedItems = smartCopy(items).sort((a, b) => a.id.localeCompare(b.id))
	})
</script>

<svelte:window
	onfocus={() => {
		if ($moveMode === 'insert') {
			$moveMode = 'move'
		}
	}}
	onkeydown={handleKeyDown}
	onkeyup={handleKeyUp}
/>

<div
	class="svlt-grid-container"
	style="height: {containerHeight}px"
	bind:this={container}
	id={root ? 'root-grid' : undefined}
	data-xperpx={xPerPx}
>
	<!-- ROOT SHADOW-->
	{#if $moveMode === 'insert' && root && $overlappedStore !== $componentDraggedParentIdStore}
		<div
			class={twMerge(
				'absolute inset-0  flex-col rounded-md bg-blue-100 dark:bg-gray-800 bg-opacity-50',
				'outline-dashed outline-offset-2 outline-2 outline-blue-300 dark:outline-blue-700',
				$componentDraggedIdStore && $overlappedStore === undefined
					? 'bg-draggedover dark:bg-draggedover-dark'
					: ''
			)}
		></div>
		{#if $overlappedStore === undefined && $componentDraggedIdStore && $fakeShadowStore}
			{@const columnGap = gapX}
			<!-- gap between the columns in px -->
			{@const containerBorder = 0.5 * 16}
			<!-- 0.5rem converted to px (1rem = 16px) -->
			{@const gridTotalWidth = containerWidth ? containerWidth - 2 * containerBorder : 0}
			<!-- subtract borders -->
			{@const availableWidth = gridTotalWidth - 11 * columnGap}
			<!-- subtract gaps between the 12 columns (11 gaps) -->
			{@const columnWidthPx = availableWidth / 12}
			<!-- divide by the number of columns -->
			{@const maxX = Math.floor(availableWidth / columnWidthPx) - $fakeShadowStore.w}

			<div class="absolute inset-0">
				<div class="relative h-full w-full">
					<div
						class="absolute bg-blue-300 transition-all"
						style={`
								left:${Math.min(maxX, $fakeShadowStore.x) * xPerPx + gapX}px ;
								top: ${$fakeShadowStore.y * yPerPx + gapY}px;
								width: ${$fakeShadowStore.w * xPerPx - gapX}px;
								height: ${$fakeShadowStore.h * yPerPx - gapY}px;
							`}
					></div>
				</div>
			</div>
		{/if}
	{/if}
	{#if xPerPx > 0 && getComputedCols}
		{#each sortedItems as item (item.id)}
			{#if item[getComputedCols] != undefined}
				{#if $moveMode === 'insert' && item.id === $overlappedStore && $componentDraggedIdStore && $componentDraggedParentIdStore !== item.id && $fakeShadowStore}
					{@const columnGap = gapX}
					<!-- gap between the columns in px -->
					{@const containerBorder = 0.5 * 16}
					<!-- 0.5rem converted to px (1rem = 16px) -->
					{@const gridTotalWidth = containerWidth ? containerWidth - 2 * containerBorder : 0}
					<!-- subtract borders -->
					{@const availableWidth = gridTotalWidth - 11 * columnGap}
					<!-- subtract gaps between the 12 columns (11 gaps) -->
					{@const columnWidthPx = availableWidth / 12}
					<!-- divide by the number of columns -->
					{@const maxX = Math.floor(availableWidth / columnWidthPx) - $fakeShadowStore.w}

					<div
						class="absolute"
						style={`
						left: ${item[getComputedCols].x * xPerPx + gapX}px;
						top: ${item[getComputedCols].y * yPerPx + gapY}px;
					`}
					>
						<div class="relative h-full w-full">
							<div
								class={twMerge('absolute transition-all duration-[50ms] bg-blue-300')}
								style={`
								left: calc(${
									Math.min($fakeShadowStore.x, maxX) * $fakeShadowStore.xPerPx + gapX
								}px + 0.5rem + ${getDeltaXByComponent(item.data['type'])});
								top: calc(${
									$fakeShadowStore.y * $fakeShadowStore.yPerPx + gapY
								}px + 0.5rem + ${getDeltaYByComponent(item.data['type'])});
								width: ${$fakeShadowStore.w * $fakeShadowStore.xPerPx - gapX * 2}px;
								height: ${$fakeShadowStore.h * $fakeShadowStore.yPerPx - gapY * 2}px;
							`}
							></div>
						</div>
					</div>
				{/if}
				<MoveResize
					{mounted}
					on:initmove={() => handleInitMove(item.id)}
					onMove={handleMove}
					bind:shadow={shadows[item.id]}
					bind:this={moveResizes[item.id]}
					on:repaint={handleRepaint}
					on:resizeStart={() => (resizing = true)}
					on:resizeEnd={() => (resizing = false)}
					onTop={Boolean(allIdsInPath?.includes(item.id))}
					id={item.id}
					{xPerPx}
					{yPerPx}
					fakeShadow={$fakeShadowStore}
					on:dropped={(e) => {
						$componentDraggedIdStore = undefined
						$componentDraggedParentIdStore = undefined
						$overlappedStore = undefined
						$fakeShadowStore = undefined
						lastDetail = undefined

						if ($moveMode === 'move') {
							return
						}

						dispatch('dropped', e.detail)
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
					type={item.data['type']}
					{disableMove}
				>
					{#if item[getComputedCols]}
						{@render children?.({
							dataItem: item,
							hidden: false,
							overlapped: $overlappedStore,
							componentDraggedId: $componentDraggedIdStore
						})}
					{/if}
				</MoveResize>
			{/if}
		{/each}
	{:else if root}
		<div
			class="h-full w-full flex-col animate-skeleton dark:bg-frost-900/50 [animation-delay:1000ms]"
		></div>
	{/if}
</div>

<style>
	.svlt-grid-container {
		position: relative;
		width: 100%;
		user-select: none;
	}
</style>
