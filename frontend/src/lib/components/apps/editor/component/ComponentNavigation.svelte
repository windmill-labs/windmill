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
		localStorage.setItem('copiedGridItem', $selectedComponent)
		localStorage.removeItem('cutGridItem')
	}

	function handleCut(event: KeyboardEvent) {
		if (!$selectedComponent) {
			return
		}

		const gridItem = findGridItem($app, $selectedComponent)

		if (!gridItem) {
			return
		}

		localStorage.setItem('cutGridItem', JSON.stringify(gridItem))
		localStorage.removeItem('copiedGridItem')
		const parent = $focusedGrid ? $focusedGrid.parentComponentId : undefined

		deleteGridItem($app, gridItem.data, parent, true)

		$app = { ...$app }
	}

	function handlePaste(event: KeyboardEvent) {
		const copied = localStorage.getItem('copiedGridItem')
		const cut = localStorage.getItem('cutGridItem')

		if (copied) {
			const parent = $focusedGrid ? $focusedGrid.parentComponentId : undefined
			duplicateGridItem($app, parent, copied)
		} else if (cut) {
			const gridItem = JSON.parse(cut)
			insertNewGridItem($app, gridItem.data, $focusedGrid, true)
		}

		$app = { ...$app }

		localStorage.removeItem('copiedGridItem')
		localStorage.removeItem('cutGridItem')
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
