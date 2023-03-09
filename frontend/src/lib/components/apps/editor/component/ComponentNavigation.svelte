<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext, EditorBreakpoint, GridItem } from '../../types'
	import { findGridItemParentId } from '../appUtils'

	const { app, selectedComponent, breakpoint, componentControl, focusedGrid } =
		getContext<AppEditorContext>('AppEditorContext')

	function getSortedGridItemsOfChildren(): GridItem[] {
		if (!$focusedGrid) {
			return sortGridItems($app.grid, $breakpoint)
		}

		if (!$app.subgrids) {
			return []
		}

		return sortGridItems(
			$app.subgrids[`${$focusedGrid.parentComponentId}-${$focusedGrid.subGridIndex}`],
			$breakpoint
		)
	}

	function getSortedGridItemsOfGrid(): GridItem[] {
		if ($app.grid.find((item) => item.id === $selectedComponent)) {
			return sortGridItems($app.grid, $breakpoint)
		}

		if (!$app.subgrids) {
			return []
		}

		return sortGridItems(
			Object.values($app.subgrids ?? {}).find((grid) =>
				grid.find((item) => item.id === $selectedComponent)
			) ?? [],
			$breakpoint
		)
	}

	function left(event) {
		if (!$componentControl[$selectedComponent!]?.left?.()) {
			const sortedGridItems = getSortedGridItemsOfGrid()
			const currentIndex = sortedGridItems.findIndex((item) => item.id === $selectedComponent)

			if (currentIndex !== -1 && currentIndex > 0) {
				$selectedComponent = sortedGridItems[currentIndex - 1].id
			}
			event.preventDefault()
		} else {
			event.preventDefault()
		}
	}

	function right(event) {
		let r = $componentControl[$selectedComponent!]?.right?.()
		if (!r) {
			const sortedGridItems = getSortedGridItemsOfGrid()
			const currentIndex = sortedGridItems.findIndex((item) => item.id === $selectedComponent)

			if (currentIndex !== -1 && currentIndex < sortedGridItems.length - 1) {
				$selectedComponent = sortedGridItems[currentIndex + 1].id
			}
			event.preventDefault()
		} else {
			event.preventDefault()
		}
	}

	function down(event) {
		if (!$focusedGrid) {
			$selectedComponent = getSortedGridItemsOfChildren()[0]?.id
		} else if ($app.subgrids) {
			const index = $focusedGrid?.subGridIndex ?? 0
			const subgrid = $app.subgrids[`${$selectedComponent}-${index}`]

			if (!subgrid || subgrid.length === 0) {
				return
			}

			const sortedGridItems = sortGridItems(subgrid, $breakpoint)

			if (sortedGridItems) {
				$selectedComponent = sortedGridItems[0].id
			}
			event.preventDefault()
		}
	}

	function keydown(event: KeyboardEvent) {
		switch (event.key) {
			case 'Escape':
				$selectedComponent = undefined
				event.preventDefault()
				break

			case 'ArrowUp': {
				if (!$selectedComponent) return
				let parentId = findGridItemParentId($app, $selectedComponent)
				if (parentId) {
					$selectedComponent = parentId
				} else {
					$selectedComponent = undefined
					$focusedGrid = undefined
				}
				break
			}

			case 'ArrowDown': {
				down(event)
				break
			}

			case 'ArrowRight': {
				right(event)
				break
			}

			case 'ArrowLeft': {
				left(event)
				break
			}

			default:
				break
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
