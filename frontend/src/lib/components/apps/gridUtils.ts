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

function toggleFixed(component: GridItem): GridItem {
	gridColumns.forEach((column: number) => {
		component[column].fixed = !component[column].fixed
	})

	return component
}

function isFixed(component: GridItem): boolean {
	let fixed = false
	gridColumns.forEach((column: number) => {
		if (component[column].fixed) {
			fixed = true
		}
	})
	return fixed
}

function enableDrag(component: GridItem): GridItem {
	gridColumns.forEach((column: number) => {
		component[column].customDragger = false
		component[column].customResizer = false
	})
	return component
}

export {
	gridColumns,
	columnConfiguration,
	disableDrag,
	enableDrag,
	Breakpoints,
	toggleFixed,
	isFixed
}
