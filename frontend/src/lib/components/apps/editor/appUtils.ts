import { getNextId } from '$lib/components/flows/flowStateUtils'
import type { App, EditorBreakpoint, FocusedGrid, GridItem } from '../types'
import { getRecommendedDimensionsByComponent, type AppComponent } from './component'
import gridHelp from '@windmill-labs/svelte-grid/src/utils/helper'
import { gridColumns } from '../gridUtils'
import { allItems } from '../utils'
import type { Output, World } from '../rx'

function findGridItemById(
	root: GridItem[],
	subGrids: Record<string, GridItem[]> | undefined,
	id: string
): GridItem | undefined {
	for (const gridItem of allItems(root, subGrids)) {
		if (gridItem.id === id) {
			return gridItem
		}
	}
	return undefined
}

export function findGridItemParentGrid(app: App, id: string): string | undefined {
	const gridItem = app.grid.find((x) => x.id === id)
	if (gridItem) {
		return undefined
	} else {
		for (const key in app.subgrids) {
			const subGrid = app.subgrids[key]
			const gridItem = subGrid.find((x) => x.id === id)
			if (gridItem) {
				return key
			}
		}
	}
}

export function findGridItem(app: App, id: string): GridItem | undefined {
	return findGridItemById(app.grid, app.subgrids, id)
}

export function getNextGridItemId(app: App): string {
	const subgridsKeys = allItems(app.grid, app.subgrids).map((x) => x.id)
	const withoutDash = subgridsKeys.map((element) => element.split('-')[0])
	const id = getNextId([...new Set(withoutDash)])

	return id
}

export function createNewGridItem(grid: GridItem[], id: string, data: AppComponent): GridItem {

	const newComponent = {
		resizable: true,
		draggable: true,
		x: 0,
		y: 0
	}

	let newData: AppComponent = JSON.parse(JSON.stringify(data))
	newData.id = id

	const newItem: GridItem = {
		data: newData,
		id: id
	}

	gridColumns.forEach((column) => {
		const rec = getRecommendedDimensionsByComponent(newData.type, column)

		newItem[column] = {
			...newComponent,
			min: { w: 1, h: 1 },
			max: { w: column, h: 100 },
			w: rec.w,
			h: rec.h,
			customDragger: false,
			customResizer: false,
			fixed: false
		}
		const position = gridHelp.findSpace(newItem, grid, column) as { x: number; y: number }
		newItem[column] = { ...newItem[column], ...position }
	})

	return newItem
}

export function getGridItems(app: App, focusedGrid: FocusedGrid | undefined): GridItem[] {
	if (!focusedGrid) {
		return app.grid
	} else {
		const { parentComponentId, subGridIndex } = focusedGrid
		const key = `${parentComponentId}-${subGridIndex ?? 0}`
		return app?.subgrids?.[key] ?? []
	}
}

export function insertNewGridItem(
	app: App,
	data: AppComponent,
	focusedGrid: FocusedGrid | undefined,
	keepId?: boolean
) {
	const id = keepId ? data.id : getNextGridItemId(app)

	if (!app.subgrids) {
		app.subgrids = {}
	}


	// We only want to set subgrids when we are not moving
	if (!keepId) {
		for (let i = 0; i < (data.numberOfSubgrids ?? 0); i++) {
			app.subgrids[`${id}-${i}`] = []
		}
	}


	const key = focusedGrid ? `${focusedGrid?.parentComponentId}-${focusedGrid?.subGridIndex ?? 0}` : undefined
	let grid = focusedGrid ? app.subgrids[key!] : app.grid

	const newItem = createNewGridItem(grid, id, data)
	grid.push(newItem)

	if (focusedGrid) {
		app.subgrids[key!] = grid
	}
	return id
}

export function getAllSubgridsAndComponentIds(
	app: App,
	component: AppComponent
): [string[], string[]] {
	const subgrids: string[] = []
	let components: string[] = [component.id]
	if (app.subgrids && component.numberOfSubgrids) {
		for (let i = 0; i < component.numberOfSubgrids; i++) {
			const key = `${component.id}-${i}`
			const subgrid = app.subgrids[key]
			if (subgrid) {
				subgrids.push(key)
				for (const item of subgrid) {
					let [recSubgrids, recComponents] = getAllSubgridsAndComponentIds(app, item.data)
					subgrids.push(...recSubgrids)
					components.push(...recComponents)
				}
			}
		}
	}
	return [subgrids, components]
}

export function deleteGridItem(
	app: App,
	component: AppComponent,
	parent: string | undefined,
	shouldKeepSubGrid: boolean
): string[] {
	let [subgrids, components] = getAllSubgridsAndComponentIds(app, component)
	if (app.subgrids && !shouldKeepSubGrid) {
		subgrids.forEach((id) => {
			delete app.subgrids![id]
		})
	}
	if (!parent) {
		let index = app.grid.findIndex((x) => x.id == component.id)
		if (index > -1) {
			app.grid.splice(index, 1)
		}
	} else {
		let grid = app.subgrids![parent]
		let index = grid.findIndex((x) => x.id == component.id)
		if (index > -1) {
			grid.splice(index, 1)
		}
	}

	return components
}



type AvailableSpace = {
	left: number
	right: number
	top: number
	bottom: number
}

export function findAvailableSpace(
	grid: GridItem[],
	gridItem: GridItem,
	editorBreakpoint: EditorBreakpoint,
	parentGridItem: GridItem | undefined = undefined
): AvailableSpace | undefined {
	if (gridItem) {
		const breakpoint = editorBreakpoint === 'sm' ? 3 : 12
		const maxHeight = parentGridItem ? parentGridItem[breakpoint].h - 1 : 16
		const maxWidth = 12

		const availableSpace = {
			left: 0,
			right: 0,
			top: 0,
			bottom: 0
		}

		const items = grid.map((item) => {
			return {
				id: item.id,
				x: item[breakpoint].x,
				y: item[breakpoint].y,
				w: item[breakpoint].w,
				h: item[breakpoint].h
			}
		})

		const item = items.find((item) => item.id === gridItem.id)

		if (!item) {
			return availableSpace
		}

		if (item.x > 0) {
			for (let x = item.x - 1; x >= 0; x--) {
				const itemToCheck = { ...item, x, w: 1 }
				const isItemInWay = items.some((item) => isOverlapping(item, itemToCheck))

				if (isItemInWay) {
					break
				} else {
					availableSpace.left++
				}
			}
		}

		if (item.x + item.w < maxWidth) {
			for (let x = item.x + item.w; x < maxWidth; x++) {
				const itemToCheck = { ...item, x, w: 1 }
				const isItemInWay = items.some((item) => isOverlapping(item, itemToCheck))

				if (isItemInWay) {
					break
				} else {
					availableSpace.right++
				}
			}
		}

		if (item.y > 0) {
			for (let y = item.y - 1; y >= 0; y--) {
				const itemToCheck = { ...item, h: 1, y }
				const isItemInWay = items.some((item) => isOverlapping(item, itemToCheck))

				if (isItemInWay) {
					break
				} else {
					availableSpace.top++
				}
			}
		}

		if (item.y + item.h < maxHeight) {
			for (let y = item.y + item.h; y < maxHeight; y++) {
				const itemToCheck = { ...item, h: 1, y }
				const isItemInWay = items.some((item) => isOverlapping(item, itemToCheck))

				if (isItemInWay) {
					break
				} else {
					availableSpace.bottom++
				}
			}
		}

		return availableSpace
	}
}

function isOverlapping(item1: any, item2: any) {
	return (
		item1.x < item2.x + item2.w &&
		item1.x + item1.w > item2.x &&
		item1.y < item2.y + item2.h &&
		item1.y + item1.h > item2.y
	)
}

type Outputtable<Type> = {
	-readonly [Property in keyof Type]: Output<Type[Property]>;
};


export function initOutput<I extends Record<string, any>>(world: World, id: string, init: I): Outputtable<I> {
	const output = world.outputsById[id] as Outputtable<I>
	if (init) {
		for (const key in init) {
			if (output && output[key] && output[key].peak() == undefined) {
				output[key].set(init[key] as any)
			}
		}
	}
	return output
}
export function expandGriditem(
	grid: GridItem[],
	gridComponent: GridItem,
	$breakpoint: EditorBreakpoint,
	parentGridItem: GridItem | undefined = undefined
) {
	const availableSpace = findAvailableSpace(grid, gridComponent, $breakpoint, parentGridItem)

	if (!availableSpace) {
		return
	}

	const { left, right, top, bottom } = availableSpace
	const width = $breakpoint === 'sm' ? 3 : 12
	const item = gridComponent[width]

	item.x = item.x - left
	item.y = item.y - top
	item.w = item.w + left + right
	item.h = item.h + top + bottom
}
