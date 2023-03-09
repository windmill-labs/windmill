<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext, EditorBreakpoint, GridItem } from '../../types'
	import { findGridItem } from '../appUtils'

	export let parentId: string | undefined = undefined
	export let subGrid: GridItem[] | undefined = undefined

	const { app, selectedComponent, breakpoint } = getContext<AppEditorContext>('AppEditorContext')

	function keydown(event: KeyboardEvent) {
		if (!$selectedComponent) {
			return
		}

		const gridItem = findGridItem($app, $selectedComponent)

		// With subgrids, we need to check if the selected component is the subgrid itself
		if (subGrid && !subGrid.find((item) => item.id === $selectedComponent)) {
			return
		}

		if (gridItem) {
			if (event.key === 'Escape') {
				$selectedComponent = undefined
			} else if (event.key === 'ArrowUp' && parentId) {
				$selectedComponent = parentId
			} else if (
				$app.subgrids &&
				event.key === 'ArrowDown' &&
				gridItem.data.numberOfSubgrids &&
				gridItem.data.numberOfSubgrids >= 1
			) {
				const subgrid = $app.subgrids[`${gridItem.data.id}-0`]
				if (subgrid) {
					const sortedGridItems = sortGridItems(subgrid, $breakpoint)
					if (sortedGridItems) {
						$selectedComponent = sortedGridItems[0].id
					}
				}
			} else {
				const grid = subGrid ?? $app.grid

				const sortedGridItems = sortGridItems(grid, $breakpoint)
				const currentIndex = sortedGridItems.findIndex((item) => item.id === $selectedComponent)

				if (event.key === 'ArrowRight') {
					if (currentIndex !== -1 && currentIndex < sortedGridItems.length - 1) {
						$selectedComponent = sortedGridItems[currentIndex + 1].id
					}
				} else if (event.key === 'ArrowLeft') {
					if (currentIndex !== -1 && currentIndex > 0) {
						$selectedComponent = sortedGridItems[currentIndex - 1].id
					}
				}
			}
		}
	}

	function sortGridItems(gridItems: GridItem[], breakpoint: EditorBreakpoint): GridItem[] {
		return gridItems.sort((a: GridItem, b: GridItem) => {
			const width = breakpoint === 'lg' ? 12 : 3

			const aX = a[width].x
			const aY = a[width].y
			const bX = b[width].x
			const bY = b[width].y

			if (aY < bY) {
				return -1
			} else if (aY > bY) {
				return 1
			} else {
				if (aX < bX) {
					return -1
				} else if (aX > bX) {
					return 1
				} else {
					return 0
				}
			}
		})
	}
</script>

<svelte:window on:keydown={keydown} />
