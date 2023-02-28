import { getNextId } from '$lib/components/flows/flowStateUtils'
import type { App, FocusedGrid, GridItem } from '../types'
import { getRecommendedDimensionsByComponent, type AppComponent } from './component'
import gridHelp from '@windmill-labs/svelte-grid/src/utils/helper'
import { gridColumns } from '../gridUtils'
import { allItems } from '../utils'

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
	const appComponent = data

	appComponent.id = id

	const newComponent = {
		fixed: false,
		resizable: true,
		draggable: true,
		customDragger: false,
		customResizer: false,
		x: 0,
		y: 0
	}

	let newData: AppComponent = JSON.parse(JSON.stringify(appComponent))

	const newItem: GridItem = {
		data: newData,
		id: id
	}

	gridColumns.forEach((column) => {
		const rec = getRecommendedDimensionsByComponent(appComponent.type, column)

		newItem[column] = {
			...newComponent,
			min: { w: 1, h: 1 },
			max: { w: column, h: 100 },
			w: rec.w,
			h: rec.h
		}
		const position = gridHelp.findSpace(newItem, grid, column) as { x: number; y: number }
		newItem[column] = { ...newItem[column], ...position }
	})

	return newItem
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

	if (!focusedGrid) {
		const newItem = createNewGridItem(app.grid, id, data)
		app.grid.push(newItem)
	} else {
		const { parentComponentId, subGridIndex } = focusedGrid

		const key = `${parentComponentId}-${subGridIndex ?? 0}`

		const subGrid = app.subgrids[key] ?? []
		subGrid.push(createNewGridItem(subGrid, id, data))
		app.subgrids[key] = subGrid
	}
	// We only want to set subgrids when we are not moving
	if (!keepId) {
		for (let i = 0; i < (data.numberOfSubgrids ?? 0); i++) {
			app.subgrids[`${id}-${i}`] = []
		}
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

export function duplicateGridItem(
	app: App,
	parent: string | undefined,
	id: string
): string | undefined {
	const gridItem = findGridItem(app, id)

	if (gridItem) {
		const newId = getNextGridItemId(app)
		const newItem = JSON.parse(JSON.stringify(gridItem))
		newItem.id = newId
		newItem.data.id = newId

		let focusedGrid = parent
			? { parentComponentId: parent.split('-')[0], subGridIndex: Number(parent.split('-')[1]) }
			: undefined

		return insertNewGridItem(app, newItem.data, focusedGrid)
	}
	return undefined
}
