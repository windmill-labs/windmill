<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext, EditorBreakpoint, GridItem } from '../../types'
	import {
		deleteGridItem,
		duplicateGridItem,
		findGridItem,
		findGridItemParentId,
		insertNewGridItem
	} from '../appUtils'

	const { app, selectedComponent, breakpoint, componentControl, focusedGrid } =
		getContext<AppEditorContext>('AppEditorContext')

	let tempGridItem: GridItem | undefined = undefined
	let copyStarted: boolean = false

	function getSortedGridItemsOfChildren(): GridItem[] {
		if (!$focusedGrid) {
			return sortGridItemsPosition($app.grid, $breakpoint)
		}

		if (!$app.subgrids) {
			return []
		}

		return sortGridItemsPosition(
			$app.subgrids[`${$focusedGrid.parentComponentId}-${$focusedGrid.subGridIndex}`],
			$breakpoint
		)
	}

	function getSortedGridItemsOfGrid(): GridItem[] {
		if ($app.grid.find((item) => item.id === $selectedComponent)) {
			return sortGridItemsPosition($app.grid, $breakpoint)
		}

		if (!$app.subgrids) {
			return []
		}

		return sortGridItemsPosition(
			Object.values($app.subgrids ?? {}).find((grid) =>
				grid.find((item) => item.id === $selectedComponent)
			) ?? [],
			$breakpoint
		)
	}

	function left(event: KeyboardEvent) {
		if (!$componentControl[$selectedComponent!]?.left?.()) {
			const sortedGridItems = getSortedGridItemsOfGrid()
			const currentIndex = sortedGridItems.findIndex((item) => item.id === $selectedComponent)

			if (currentIndex !== -1 && currentIndex > 0) {
				$selectedComponent = sortedGridItems[currentIndex - 1].id
			}
		}

		event.preventDefault()
	}

	function right(event: KeyboardEvent) {
		let r = $componentControl[$selectedComponent!]?.right?.()
		if (!r) {
			const sortedGridItems = getSortedGridItemsOfGrid()
			const currentIndex = sortedGridItems.findIndex((item) => item.id === $selectedComponent)

			if (currentIndex !== -1 && currentIndex < sortedGridItems.length - 1) {
				$selectedComponent = sortedGridItems[currentIndex + 1].id
			}
		}

		event.preventDefault()
	}

	function down(event: KeyboardEvent) {
		if (!$focusedGrid) {
			$selectedComponent = getSortedGridItemsOfChildren()[0]?.id
			event.preventDefault()
		} else if ($app.subgrids) {
			const index = $focusedGrid?.subGridIndex ?? 0
			const subgrid = $app.subgrids[`${$selectedComponent}-${index}`]

			if (!subgrid || subgrid.length === 0) {
				return
			}

			const sortedGridItems = sortGridItemsPosition(subgrid, $breakpoint)

			if (sortedGridItems) {
				$selectedComponent = sortedGridItems[0].id
			}
			event.preventDefault()
		}
	}

	function handleEscape(event: KeyboardEvent) {
		$selectedComponent = undefined
		event.preventDefault()
	}

	function handleArrowUp(event: KeyboardEvent) {
		if (!$selectedComponent) return
		let parentId = findGridItemParentId($app, $selectedComponent)
		if (parentId) {
			$selectedComponent = parentId
		} else {
			$selectedComponent = undefined
			$focusedGrid = undefined
		}
	}

	function handleCopy(event: KeyboardEvent) {
		if (!$selectedComponent) {
			return
		}

		copyStarted = true
	}

	function handleCut(event: KeyboardEvent) {
		copyStarted = false
		if (!$selectedComponent) {
			return
		}

		const gridItem = findGridItem($app, $selectedComponent)

		if (!gridItem) {
			return
		}

		// Store the grid item in a temp variable so we can paste it later
		tempGridItem = gridItem

		const parent = $focusedGrid ? $focusedGrid.parentComponentId : undefined

		deleteGridItem($app, gridItem.data, parent, true)

		$app = { ...$app }
	}

	function handlePaste(event: KeyboardEvent) {
		if (copyStarted && $selectedComponent) {
			const parent = $focusedGrid ? $focusedGrid.parentComponentId : undefined
			duplicateGridItem($app, parent, $selectedComponent)
		} else if (tempGridItem) {
			insertNewGridItem($app, tempGridItem.data, $focusedGrid, true)
			tempGridItem = undefined
		}

		$app = { ...$app }
	}

	function keydown(event: KeyboardEvent) {
		switch (event.key) {
			case 'Escape':
				handleEscape(event)
				break

			case 'ArrowUp': {
				handleArrowUp(event)
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

			case 'c':
				if (event.ctrlKey) {
					handleCopy(event)
				}
				break

			case 'v':
				if (event.ctrlKey) {
					handlePaste(event)
				}
				break

			case 'x':
				if (event.ctrlKey) {
					handleCut(event)
				}
				break

			default:
				break
		}
	}

	function sortGridItemsPosition(gridItems: GridItem[], breakpoint: EditorBreakpoint): GridItem[] {
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
