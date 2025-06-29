import { writable } from 'svelte/store'
import type { GridItem } from './types'

type ColumnConfiguration = [number, number][]

const Breakpoints = {
	sm: 640,
	lg: 1024
}

export const moveMode = writable<'move' | 'insert'>('move')


const WIDE_GRID_COLUMNS = 12 as const
const NARROW_GRID_COLUMNS = 3 as const

const columnConfiguration: ColumnConfiguration = [
	[Breakpoints.lg, WIDE_GRID_COLUMNS],
	[Breakpoints.sm, NARROW_GRID_COLUMNS]
]

const gridColumns = columnConfiguration.map((value) => value[1])

function toggleFixed(component: GridItem): GridItem {
	const nValue = !(component[gridColumns[0]]?.fixed ?? false)
	gridColumns.forEach((column: number) => {
		component[column].fixed = nValue
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

export {
	gridColumns,
	WIDE_GRID_COLUMNS,
	NARROW_GRID_COLUMNS,
	columnConfiguration,
	Breakpoints,
	toggleFixed,
	isFixed
}
