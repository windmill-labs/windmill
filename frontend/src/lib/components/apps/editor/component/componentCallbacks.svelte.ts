// import { getContext } from 'svelte'
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
import { get } from 'svelte/store'

// const { app, selectedComponent, focusedGrid, componentControl } =
// 	getContext<AppViewerContext>('AppViewerContext')

// const { history, movingcomponents, jobsDrawerOpen } =
// 	getContext<AppEditorContext>('AppEditorContext')

let tempGridItems: GridItem[] | undefined = undefined

const ITEM_TYPE = 'wm-grid-items'

function getSortedGridItemsOfChildren(ctx: {
	app: AppViewerContext['app']
	focusedGrid: AppViewerContext['focusedGrid']
}): GridItem[] {
	const focusedGrid = get(ctx.focusedGrid)
	if (!focusedGrid) {
		return ctx.app.val.grid
	}

	let subgrids = ctx.app.val.subgrids
	if (!subgrids) {
		return []
	}

	return subgrids[`${focusedGrid.parentComponentId}-${focusedGrid.subGridIndex}`] ?? []
}

function getGridItems(ctx: {
	app: AppViewerContext['app']
	selectedComponent: AppViewerContext['selectedComponent']
}): GridItem[] {
	const app = ctx.app.val
	const selectedComponent = get(ctx.selectedComponent)
	if (app.grid.find((item) => item.id === selectedComponent?.[0])) {
		return ctx.app.val.grid
	}

	if (!app.subgrids) {
		return []
	}

	return (
		Object.values(app.subgrids ?? {}).find((grid) =>
			grid.find((item) => item.id === selectedComponent?.[0])
		) ?? []
	)
}

export function left(
	event: KeyboardEvent,
	ctx: {
		app: AppViewerContext['app']
		componentControl: AppViewerContext['componentControl']
		selectedComponent: AppViewerContext['selectedComponent']
	}
) {
	const componentControl = get(ctx.componentControl)
	const selectedComponent = get(ctx.selectedComponent)
	if (!componentControl[selectedComponent?.[0] ?? '']?.left?.()) {
		const sortedGridItems = getGridItems(ctx)
		const currentIndex = sortedGridItems.findIndex(
			(item) => item.id === (selectedComponent?.[0] ?? '')
		)

		if (currentIndex !== -1 && currentIndex > 0) {
			const left = sortedGridItems[currentIndex - 1]

			if (left.data.type === 'tablecomponent' && left.data.actionButtons.length >= 1) {
				ctx.selectedComponent.set([left.data.actionButtons[left.data.actionButtons.length - 1].id])
			} else if (
				(left.data.type === 'aggridcomponent' ||
					left.data.type === 'aggridcomponentee' ||
					left.data.type === 'dbexplorercomponent') &&
				Array.isArray(left.data.actions) &&
				left.data.actions.length >= 1
			) {
				ctx.selectedComponent.set([left.data.actions[left.data.actions.length - 1].id])
			} else {
				ctx.selectedComponent.set([left.id])
			}
		}
	}

	event.preventDefault()
}

export function right(
	event: KeyboardEvent,
	ctx: {
		app: AppViewerContext['app']
		componentControl: AppViewerContext['componentControl']
		selectedComponent: AppViewerContext['selectedComponent']
	}
) {
	const componentControl = get(ctx.componentControl)
	const selectedComponent = get(ctx.selectedComponent)
	let r = componentControl[selectedComponent?.[0] ?? '']?.right?.()

	if (typeof r === 'string') {
		ctx.selectedComponent.set([r])
		r = componentControl[r]?.right?.(true)
	}

	if (!r) {
		const sortedGridItems = getGridItems(ctx)
		const currentIndex = sortedGridItems.findIndex(
			(item) => item.id === (selectedComponent?.[0] ?? '')
		)

		if (currentIndex !== -1 && currentIndex < sortedGridItems.length - 1) {
			ctx.selectedComponent.set([sortedGridItems[currentIndex + 1].id])
		}
	}

	event.preventDefault()
}

export function down(
	event: KeyboardEvent,
	ctx: {
		app: AppViewerContext['app']
		componentControl: AppViewerContext['componentControl']
		selectedComponent: AppViewerContext['selectedComponent']
		focusedGrid: AppViewerContext['focusedGrid']
	}
) {
	const focusedGrid = get(ctx.focusedGrid)
	const selectedComponent = get(ctx.selectedComponent)
	const app = ctx.app.val
	if (!focusedGrid) {
		ctx.selectedComponent.set([getSortedGridItemsOfChildren(ctx)[0]?.id])
		event.preventDefault()
	} else if (app.subgrids) {
		const index = focusedGrid?.subGridIndex ?? 0
		const subgrid = app.subgrids[`${selectedComponent}-${index}`]

		if (!subgrid || subgrid.length === 0) {
			return
		}

		if (subgrid) {
			ctx.selectedComponent.set([subgrid[0].id])
		}
		event.preventDefault()
	}
}

export function handleEscape(
	event: KeyboardEvent,
	ctx: {
		selectedComponent: AppViewerContext['selectedComponent']
		focusedGrid: AppViewerContext['focusedGrid']
	}
) {
	ctx.selectedComponent.set(undefined)
	ctx.focusedGrid.set(undefined)
	event.preventDefault()
}

export function handleArrowUp(
	event: KeyboardEvent,
	ctx: {
		app: AppViewerContext['app']
		selectedComponent: AppViewerContext['selectedComponent']
		focusedGrid: AppViewerContext['focusedGrid']
	}
) {
	const selectedComponent = get(ctx.selectedComponent)
	if (!selectedComponent) return
	let parentId = findGridItemParentGrid(ctx.app.val, selectedComponent?.[0])?.split('-')[0]

	if (parentId) {
		ctx.selectedComponent.set([parentId])
	} else {
		ctx.selectedComponent.set(undefined)
		ctx.focusedGrid.set(undefined)
	}
}

export async function handleCopy(
	event: KeyboardEvent,
	ctx: {
		app: AppViewerContext['app']
		selectedComponent: AppViewerContext['selectedComponent']
		jobsDrawerOpen: AppEditorContext['jobsDrawerOpen']
	}
) {
	const selectedComponent = get(ctx.selectedComponent)
	if (!selectedComponent || get(ctx.jobsDrawerOpen) || window.getSelection()?.toString() != '') {
		return
	}
	tempGridItems = undefined
	const copiedGridItems = selectedComponent
		.map((x) => findGridItem(ctx.app.val, x))
		.filter((x) => x != undefined) as GridItem[]

	copyGridItemsToClipboard(copiedGridItems, ctx, 'copy')
}

async function copyGridItemsToClipboard(
	items: GridItem[],
	ctx: {
		app: AppViewerContext['app']
	},
	type: 'copy' | 'cut' | undefined = undefined
) {
	const app = ctx.app.val
	let allSubgrids = {}
	for (let item of items) {
		let subgrids = getAllSubgridsAndComponentIds(app, item.data)[0]
		for (let key of subgrids) {
			allSubgrids[key] = app.subgrids?.[key]
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

export function handleCut(
	event: KeyboardEvent,
	ctx: {
		selectedComponent: AppViewerContext['selectedComponent']
		history: AppEditorContext['history']
		app: AppViewerContext['app']
		movingcomponents: AppEditorContext['movingcomponents']
	}
) {
	const selectedComponent = get(ctx.selectedComponent)
	const app = ctx.app.val
	if (!selectedComponent) {
		return
	}
	ctx.movingcomponents.set(JSON.parse(JSON.stringify(selectedComponent)))
	push(ctx.history, app)

	let gridItems = selectedComponent
		.map((x) => findGridItem(app, x))
		.filter((x) => x != undefined) as GridItem[]
	copyGridItemsToClipboard(gridItems, ctx, 'cut')

	if (!gridItems) {
		return
	}

	// Store the grid item in a temp variable so we can paste it later
	tempGridItems = gridItems
}

export async function handlePaste(
	event: ClipboardEvent,
	ctx: {
		app: AppViewerContext['app']
		selectedComponent: AppViewerContext['selectedComponent']
		focusedGrid: AppViewerContext['focusedGrid']
		componentControl: AppViewerContext['componentControl']
		history: AppEditorContext['history']
		jobsDrawerOpen: AppEditorContext['jobsDrawerOpen']
		movingcomponents: AppEditorContext['movingcomponents']
	}
) {
	let classes = event.target?.['className']
	if (
		(typeof classes === 'string' && classes.includes('inputarea')) ||
		['INPUT', 'TEXTAREA'].includes(document.activeElement?.tagName!)
	) {
		return
	}
	event.preventDefault()
	const app = ctx.app.val
	const focusedGrid = get(ctx.focusedGrid)

	push(ctx.history, app)
	ctx.movingcomponents.set(undefined)
	let copiedGridItems: GridItem[] | undefined = undefined
	let subgrids = app.subgrids ?? {}
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
				focusedGrid &&
				getAllSubgridsAndComponentIds(app, tempGridItem.data)[0].includes(
					`${focusedGrid.parentComponentId}-${focusedGrid.subGridIndex}`
				)
			) {
				sendUserToast('Cannot paste a component into itself', true)
				return
			}
			let parentGrid = findGridItemParentGrid(app, tempGridItem.id)
			if (parentGrid) {
				app.subgrids &&
					(app.subgrids[parentGrid] = app.subgrids[parentGrid].filter(
						(item) => item.id !== tempGridItem?.id
					))
			} else {
				app.grid = app.grid.filter((item) => item.id !== tempGridItem?.id)
			}

			const gridItem = tempGridItem
			insertNewGridItem(
				app,
				(id) => ({ ...gridItem.data, id }),
				focusedGrid,
				Object.fromEntries(gridColumns.map((column) => [column, gridItem[column]])),
				tempGridItem.id
			)
		}
		copyGridItemsToClipboard(tempGridItems, ctx)
		ctx.selectedComponent.set(tempGridItems.map((x) => x.id))

		tempGridItems = undefined
	} else if (copiedGridItems) {
		let nitems: string[] = []
		for (let copiedGridItem of copiedGridItems) {
			let newItem = copyComponent(app, copiedGridItem, focusedGrid, subgrids, [])
			newItem && nitems.push(newItem)
		}
		ctx.selectedComponent.set(nitems.map((x) => x))
	}

	ctx.app.val = ctx.app.val
}
