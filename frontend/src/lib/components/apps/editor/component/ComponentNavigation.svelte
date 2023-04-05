<script lang="ts">
	import { getContext } from 'svelte'
	import {
		findGridItem,
		findGridItemParentGrid,
		getAllSubgridsAndComponentIds,
		insertNewGridItem
	} from '../appUtils'
	import type { AppEditorContext, AppViewerContext, GridItem } from '../../types'
	import { push } from '$lib/history'
	import { sendUserToast } from '$lib/utils'
	import { gridColumns } from '../../gridUtils'

	const { app, selectedComponent, worldStore, focusedGrid, componentControl } =
		getContext<AppViewerContext>('AppViewerContext')

	const { history, movingcomponents } = getContext<AppEditorContext>('AppEditorContext')

	let tempGridItems: GridItem[] | undefined = undefined
	let copiedGridItems: GridItem[] | undefined = undefined

	function getSortedGridItemsOfChildren(): GridItem[] {
		if (!$focusedGrid) {
			return $app.grid
		}

		if (!$app.subgrids) {
			return []
		}

		return $app.subgrids[`${$focusedGrid.parentComponentId}-${$focusedGrid.subGridIndex}`] ?? []
	}

	function getGridItems(): GridItem[] {
		if ($app.grid.find((item) => item.id === $selectedComponent?.[0])) {
			return $app.grid
		}

		if (!$app.subgrids) {
			return []
		}

		return (
			Object.values($app.subgrids ?? {}).find((grid) =>
				grid.find((item) => item.id === $selectedComponent?.[0])
			) ?? []
		)
	}

	function left(event: KeyboardEvent) {
		if (!$componentControl[$selectedComponent?.[0] ?? '']?.left?.()) {
			const sortedGridItems = getGridItems()
			const currentIndex = sortedGridItems.findIndex(
				(item) => item.id === $selectedComponent?.[0] ?? ''
			)

			if (currentIndex !== -1 && currentIndex > 0) {
				const left = sortedGridItems[currentIndex - 1]

				if (left.data.type === 'tablecomponent' && left.data.actionButtons.length >= 1) {
					$selectedComponent = [left.data.actionButtons[left.data.actionButtons.length - 1].id]
				} else {
					$selectedComponent = [left.id]
				}
			}
		}

		event.preventDefault()
	}

	function right(event: KeyboardEvent) {
		let r = $componentControl[$selectedComponent?.[0] ?? '']?.right?.()

		if (typeof r === 'string') {
			$selectedComponent = [r]
			r = $componentControl[r]?.right?.(true)
		}

		if (!r) {
			const sortedGridItems = getGridItems()
			const currentIndex = sortedGridItems.findIndex(
				(item) => item.id === $selectedComponent?.[0] ?? ''
			)

			if (currentIndex !== -1 && currentIndex < sortedGridItems.length - 1) {
				$selectedComponent = [sortedGridItems[currentIndex + 1].id]
			}
		}

		event.preventDefault()
	}

	function down(event: KeyboardEvent) {
		if (!$focusedGrid) {
			$selectedComponent = [getSortedGridItemsOfChildren()[0]?.id]
			event.preventDefault()
		} else if ($app.subgrids) {
			const index = $focusedGrid?.subGridIndex ?? 0
			const subgrid = $app.subgrids[`${$selectedComponent}-${index}`]

			if (!subgrid || subgrid.length === 0) {
				return
			}

			if (subgrid) {
				$selectedComponent = [subgrid[0].id]
			}
			event.preventDefault()
		}
	}

	function handleEscape(event: KeyboardEvent) {
		$selectedComponent = undefined
		$focusedGrid = undefined
		event.preventDefault()
	}

	function handleArrowUp(event: KeyboardEvent) {
		if (!$selectedComponent) return
		let parentId = findGridItemParentGrid($app, $selectedComponent?.[0])?.split('-')[0]

		if (parentId) {
			$selectedComponent = [parentId]
		} else {
			$selectedComponent = undefined
			$focusedGrid = undefined
		}
	}

	function handleCopy(event: KeyboardEvent) {
		if (!$selectedComponent) {
			return
		}
		tempGridItems = undefined
		copiedGridItems = $selectedComponent
			.map((x) => findGridItem($app, x))
			.filter((x) => x != undefined) as GridItem[]
	}

	function handleCut(event: KeyboardEvent) {
		if (!$selectedComponent) {
			return
		}
		$movingcomponents = JSON.parse(JSON.stringify($selectedComponent))
		push(history, $app)

		let gridItems = $selectedComponent
			.map((x) => findGridItem($app, x))
			.filter((x) => x != undefined) as GridItem[]
		copiedGridItems = gridItems

		if (!gridItems) {
			return
		}

		// Store the grid item in a temp variable so we can paste it later
		tempGridItems = gridItems
	}

	function handlePaste(event: KeyboardEvent) {
		push(history, $app)
		$movingcomponents = undefined
		if (tempGridItems != undefined) {
			for (let tempGridItem of tempGridItems) {
				if (
					$focusedGrid &&
					getAllSubgridsAndComponentIds($app, tempGridItem.data)[0].includes(
						`${$focusedGrid.parentComponentId}-${$focusedGrid.subGridIndex}`
					)
				) {
					sendUserToast('Cannot paste a component into itself', true)
					return
				}
				let parentGrid = findGridItemParentGrid($app, tempGridItem.id)
				if (parentGrid) {
					$app.subgrids &&
						($app.subgrids[parentGrid] = $app.subgrids[parentGrid].filter(
							(item) => item.id !== tempGridItem?.id
						))
				} else {
					$app.grid = $app.grid.filter((item) => item.id !== tempGridItem?.id)
				}

				const gridItem = tempGridItem
				insertNewGridItem(
					$app,
					(id) => ({ ...gridItem.data, id }),
					$focusedGrid,
					Object.fromEntries(gridColumns.map((column) => [column, gridItem[column]])),
					tempGridItem.id
				)
			}
			copiedGridItems = tempGridItems
			$selectedComponent = tempGridItems.map((x) => x.id)

			tempGridItems = undefined
		} else if (copiedGridItems) {
			let nitems: string[] = []
			for (let copiedGridItem of copiedGridItems) {
				nitems.push(
					insertNewGridItem(
						$app,
						(id) => ({ ...copiedGridItem.data, id }),
						$focusedGrid,
						Object.fromEntries(gridColumns.map((column) => [column, copiedGridItem[column]]))
					)
				)
			}
			$selectedComponent = nitems.map((x) => x)
		}

		$worldStore = $worldStore
		$app = $app
	}

	function keydown(event: KeyboardEvent) {
		// Ignore keydown events if the user is typing in monaco
		let classes = event.target?.['className']
		if (
			(typeof classes === 'string' && classes.includes('inputarea')) ||
			['INPUT', 'TEXTAREA'].includes(document.activeElement?.tagName!)
		) {
			return
		}
		switch (event.key) {
			case 'Escape':
				handleEscape(event)
				break

			case 'ArrowUp': {
				handleArrowUp(event)
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

			case 'c':
				if (event.ctrlKey || event.metaKey) {
					handleCopy(event)
				}
				break

			case 'v':
				if (event.ctrlKey || event.metaKey) {
					handlePaste(event)
				}
				break

			case 'x':
				if (event.ctrlKey || event.metaKey) {
					handleCut(event)
				}
				break

			default:
				break
		}
	}
</script>

<svelte:window on:keydown={keydown} />
