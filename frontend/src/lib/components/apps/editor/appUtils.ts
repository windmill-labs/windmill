import { getNextId } from '$lib/components/flows/flowStateUtils'
import type { App, FocusedGrid, GridItem } from '../types'
import { getRecommendedDimensionsByComponent, type AppComponent } from './component'
import gridHelp from '@windmill-labs/svelte-grid/src/utils/helper'
import { gridColumns } from '../gridUtils'

function findGridItemById(
	root: GridItem[],
	subGrids: Record<string, GridItem[]> | undefined,
	id: string
): GridItem | undefined {
	for (const gridItem of root) {
		if (gridItem.id === id) {
			return gridItem
		}

		if (subGrids) {
			const numberOfSubgrids = gridItem.data.numberOfSubgrids
			const subgrids = subGrids[gridItem.id]
			if (numberOfSubgrids && subgrids) {
				for (let i = 0; i < numberOfSubgrids; i++) {
					const subgrid = subgrids[`${gridItem.id}-${i}`]
					const found = findGridItemById([subgrid], subGrids, id)

					if (found) {
						return found
					}
				}
			}
		}
	}

	return undefined
}

export function findGridItem(app: App, id: string): GridItem | undefined {
	return findGridItemById(app.grid, app.subgrids, id)
}

export function getNextGridItemId(app: App): string {
	const all: string[] = []

	const newArr = all.map((element) => element.split('-')[0])
	const k: string[] = [...new Set(newArr)]
	const id = getNextId(k)

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
	focusedGrid: FocusedGrid | undefined
) {
	const id = getNextGridItemId(app)

	if (!focusedGrid) {
		const newItem = createNewGridItem(app.grid, id, data)
		app.grid.push(newItem)
	} else {
		const { parentComponentId, subGridIndex } = focusedGrid

		if (!app.subgrids) {
			app.subgrids = {}
		}

		const subGrid = app.subgrids[`${parentComponentId}-${subGridIndex}`] ?? []
		const newItem = createNewGridItem(subGrid, id, data)
		const key = `${parentComponentId}-${subGridIndex ?? 0}`

		if (!app.subgrids[key]) {
			app.subgrids[key] = [newItem]
		} else {
			app.subgrids[key].push(newItem)
		}
	}

	return id
}

export function deleteGridItem(app: App, id: string) {
	const index = app.grid.findIndex((item) => item.id === id)
	if (index !== -1) {
		app.grid.splice(index, 1)
		return
	}

	const gridItem = findGridItem(app, id)
	const keys = Object.keys(app.subgrids ?? {})

	if (app.subgrids) {
		if (keys.includes(id)) {
			for (let i = 0; i < gridItem?.data.numberOfSubgrids; i++) {
				delete app.subgrids[`${id}-${i}`]
			}
		} else {
			for (const key of keys) {
				const subgrid = app.subgrids[key]
				const index = subgrid.findIndex((item) => item.id === id)
				if (index !== -1) {
					subgrid.splice(index, 1)
					return
				}
			}
		}
	}
}

export function duplicateGridItem(
	app: App,
	focusedGrid: FocusedGrid | undefined,
	id: string
): string | undefined {
	const gridItem = findGridItem(app, id)
	if (gridItem) {
		const newId = getNextGridItemId(app)
		const newItem = JSON.parse(JSON.stringify(gridItem))
		newItem.id = newId
		newItem.data.id = newId

		return insertNewGridItem(app, newItem.data, focusedGrid)
	}
	return undefined
}
