<script lang="ts">
	import { getContext } from 'svelte'
	import {
		copyComponent,
		findGridItem,
		findGridItemParentGrid,
		getAllSubgridsAndComponentIds,
		insertNewGridItem
	} from '../appUtils'
	import type { AppEditorContext, AppViewerContext, GridItem } from '../../types'
	import { push } from '$lib/history'
	import { sendUserToast } from '$lib/toast'
	import { gridColumns } from '../../gridUtils'
	import { copyToClipboard } from '$lib/utils'

	const { app, selectedComponent, focusedGrid, componentControl } =
		getContext<AppViewerContext>('AppViewerContext')

	const { history, movingcomponents, jobsDrawerOpen } =
		getContext<AppEditorContext>('AppEditorContext')

	let tempGridItems: GridItem[] | undefined = undefined

	const ITEM_TYPE = 'wm-grid-items'
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

	export function left(event: KeyboardEvent) {
		if (!$componentControl[$selectedComponent?.[0] ?? '']?.left?.()) {
			const sortedGridItems = getGridItems()
			const currentIndex = sortedGridItems.findIndex(
				(item) => item.id === ($selectedComponent?.[0] ?? '')
			)

			if (currentIndex !== -1 && currentIndex > 0) {
				const left = sortedGridItems[currentIndex - 1]

				if (left.data.type === 'tablecomponent' && left.data.actionButtons.length >= 1) {
					$selectedComponent = [left.data.actionButtons[left.data.actionButtons.length - 1].id]
				} else if (
					(left.data.type === 'aggridcomponent' ||
						left.data.type === 'aggridcomponentee' ||
						left.data.type === 'dbexplorercomponent') &&
					Array.isArray(left.data.actions) &&
					left.data.actions.length >= 1
				) {
					$selectedComponent = [left.data.actions[left.data.actions.length - 1].id]
				} else {
					$selectedComponent = [left.id]
				}
			}
		}

		event.preventDefault()
	}

	export function right(event: KeyboardEvent) {
		let r = $componentControl[$selectedComponent?.[0] ?? '']?.right?.()

		if (typeof r === 'string') {
			$selectedComponent = [r]
			r = $componentControl[r]?.right?.(true)
		}

		if (!r) {
			const sortedGridItems = getGridItems()
			const currentIndex = sortedGridItems.findIndex(
				(item) => item.id === ($selectedComponent?.[0] ?? '')
			)

			if (currentIndex !== -1 && currentIndex < sortedGridItems.length - 1) {
				$selectedComponent = [sortedGridItems[currentIndex + 1].id]
			}
		}

		event.preventDefault()
	}

	export function down(event: KeyboardEvent) {
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

	export function handleEscape(event: KeyboardEvent) {
		$selectedComponent = undefined
		$focusedGrid = undefined
		event.preventDefault()
	}

	export function handleArrowUp(event: KeyboardEvent) {
		if (!$selectedComponent) return
		let parentId = findGridItemParentGrid($app, $selectedComponent?.[0])?.split('-')[0]

		if (parentId) {
			$selectedComponent = [parentId]
		} else {
			$selectedComponent = undefined
			$focusedGrid = undefined
		}
	}

	export async function handleCopy(event: KeyboardEvent) {
		if (!$selectedComponent || $jobsDrawerOpen || window.getSelection()?.toString() != '') {
			return
		}
		tempGridItems = undefined
		const copiedGridItems = $selectedComponent
			.map((x) => findGridItem($app, x))
			.filter((x) => x != undefined) as GridItem[]

		copyGridItemsToClipboard(copiedGridItems, 'copy')
	}

	async function copyGridItemsToClipboard(
		items: GridItem[],
		type: 'copy' | 'cut' | undefined = undefined
	) {
		let allSubgrids = {}
		for (let item of items) {
			let subgrids = getAllSubgridsAndComponentIds($app, item.data)[0]
			for (let key of subgrids) {
				allSubgrids[key] = $app.subgrids?.[key]
			}
		}
		let success = await copyToClipboard(
			JSON.stringify({ type: ITEM_TYPE, items, subgrids: allSubgrids }),
			false
		)

		if (!success) {
			sendUserToast('Could not copy to clipboard. Are you in an unsecure context?', true)
		} else if (type) {
			const text = type == 'copy' ? 'Copied' : 'Cut'
			sendUserToast(`${text} ${items.length} component${items.length > 1 ? 's' : ''}`)
		}
	}

	export function handleCut(event: KeyboardEvent) {
		if (!$selectedComponent) {
			return
		}
		$movingcomponents = JSON.parse(JSON.stringify($selectedComponent))
		push(history, $app)

		let gridItems = $selectedComponent
			.map((x) => findGridItem($app, x))
			.filter((x) => x != undefined) as GridItem[]
		copyGridItemsToClipboard(gridItems, 'cut')

		if (!gridItems) {
			return
		}

		// Store the grid item in a temp variable so we can paste it later
		tempGridItems = gridItems
	}

	export async function handlePaste(event: ClipboardEvent) {
		let classes = event.target?.['className']
		if (
			(typeof classes === 'string' && classes.includes('inputarea')) ||
			['INPUT', 'TEXTAREA'].includes(document.activeElement?.tagName!)
		) {
			return
		}
		event.preventDefault()

		push(history, $app)
		$movingcomponents = undefined
		let copiedGridItems: GridItem[] | undefined = undefined
		let subgrids = $app.subgrids ?? {}
		const txt = event?.clipboardData?.getData('text')
		if (txt) {
			try {
				const parsed = JSON.parse(txt)
				if (parsed.type == ITEM_TYPE) {
					copiedGridItems = parsed.items
					subgrids = parsed.subgrids ?? subgrids
				} else {
					copiedGridItems = undefined
				}
			} catch {}
		}

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
			copyGridItemsToClipboard(tempGridItems)
			$selectedComponent = tempGridItems.map((x) => x.id)

			tempGridItems = undefined
		} else if (copiedGridItems) {
			let nitems: string[] = []
			for (let copiedGridItem of copiedGridItems) {
				let newItem = copyComponent($app, copiedGridItem, $focusedGrid, subgrids, [])
				newItem && nitems.push(newItem)
			}
			$selectedComponent = nitems.map((x) => x)
		}

		$app = $app
	}
</script>
