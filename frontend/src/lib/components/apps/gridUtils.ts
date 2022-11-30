import type { GridItem } from './types'

type ColumnConfiguration = [number, number][]

const Breakpoints = {
	sm: 640,
	lg: 1024
}

const columnConfiguration: ColumnConfiguration = [
	[Breakpoints.lg, 12],
	[Breakpoints.sm, 3]
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
