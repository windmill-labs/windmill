<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext, EditorBreakpoint, GridItem } from '../../types'
	import { findGridItemParentId } from '../appUtils'

	const { app, selectedComponent, breakpoint } = getContext<AppEditorContext>('AppEditorContext')

	function getSortedGridItems(parentId: string | undefined): GridItem[] {
		if (!parentId) {
			return sortGridItems($app.grid, $breakpoint)
		}

		if (!$app.subgrids) {
			return []
		}

		return sortGridItems($app.subgrids[`${parentId}-0`], $breakpoint)
	}

	function keydown(event: KeyboardEvent) {
		if (!$selectedComponent) {
			return
		}

		const parentId = findGridItemParentId($app, $selectedComponent)

		switch (event.key) {
			case 'Escape':
				$selectedComponent = undefined
				break

			case 'ArrowUp':
				if (parentId) {
					$selectedComponent = parentId
				}
				break

			case 'ArrowDown': {
				if ($app.subgrids) {
					const subgrid = $app.subgrids[`${$selectedComponent}-0`]

					if (!subgrid) {
						return
					}

					const sortedGridItems = sortGridItems(subgrid, $breakpoint)
					if (sortedGridItems) {
						$selectedComponent = sortedGridItems[0].id
					}
				}
				break
			}

			case 'ArrowRight': {
				const sortedGridItems = getSortedGridItems(parentId)
				const currentIndex = sortedGridItems.findIndex((item) => item.id === $selectedComponent)

				if (currentIndex !== -1 && currentIndex < sortedGridItems.length - 1) {
					$selectedComponent = sortedGridItems[currentIndex + 1].id
				}
				break
			}

			case 'ArrowLeft': {
				const sortedGridItems = getSortedGridItems(parentId)
				const currentIndex = sortedGridItems.findIndex((item) => item.id === $selectedComponent)

				if (currentIndex !== -1 && currentIndex > 0) {
					$selectedComponent = sortedGridItems[currentIndex - 1].id
				}
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
