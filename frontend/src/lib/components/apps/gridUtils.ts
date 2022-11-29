import type { GridItem } from './types'

type ColumnConfiguration = [number, number][]

const Breakpoints = {
	sm: 640,
	lg: 1024
}

const columnConfiguration: ColumnConfiguration = [
	// Starting from 1000px, the grid will have 12 columns
	[Breakpoints.lg, 12],
	// Starting from 500px, the grid will have 1 columns
	[Breakpoints.sm, 1]
]

const gridColumns = columnConfiguration.map((value) => value[1])

function disableDrag(component: GridItem): GridItem {
	gridColumns.forEach((column: number) => {
		component[column].customDragger = true
		component[column].customResizer = true
	})
	return component
}

function enableDrag(component: GridItem): GridItem {
	gridColumns.forEach((column: number) => {
		component[column].customDragger = false
		component[column].customResizer = false
	})
	return component
}

export { gridColumns, columnConfiguration, disableDrag, enableDrag, Breakpoints }
