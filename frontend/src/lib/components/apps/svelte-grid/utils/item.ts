import type { FilledItem, ItemLayout } from '../types'
import {
	makeMatrix,
	makeMatrixFromItemsIgnore,
	findCloseBlocks,
	findItemsById,
	makeMatrixFromItems
} from './matrix'
import { getRowsCount } from './other'

export function getItemById(id, items) {
	return items.find((value) => value.id === id)
}

export function isEmpty(matrix: any[][], x: number, y: number, w: number, h: number) {
	for (var i = 0; i < h; i++) {
		if (matrix[y + i]) {
			for (var j = 0; j < w; j++) {
				if (matrix[y + i][x + j] != undefined) {
					return false
				}
			}
		}
	}
	return true
}

function distance(a, b): number {
	return Math.abs(a.x - b.x + 0.25) + Math.abs(a.y - b.y + 0.25)
}

export function findFreeSpaceForItem<T>(matrix: FilledItem<T>[][], item: ItemLayout) {
	const cols = matrix[0].length

	const w = Math.min(cols, item.w)
	const h = item.h
	let xNtime = cols - w + 1
	let getMatrixRows = Math.max(matrix.length, item.y)

	const range = Array.from({ length: getMatrixRows }, (_, y) =>
		Array.from({ length: xNtime }, (_, x) => ({ x, y }))
	)
		.flat(1)
		.sort((a, b) => {
			let dst1 = distance(a, item)
			let dst2 = distance(b, item)
			if (dst1 > dst2) {
				return 1
			} else if (dst1 < dst2) {
				return -1
			} else {
				return 0
			}
		})

	for (const { x, y } of range.values()) {
		if (isEmpty(matrix, x, y, w, h)) {
			return { x, y }
		}
	}

	return {
		y: getMatrixRows,
		x: 0
	}
}

const getItem = (item, col) => {
	return { ...item[col] }
}

const updateItem = (elements, active, position, col) => {
	return elements.map((value) => {
		if (value.id === active.id) {
			return { ...value, [col]: { ...value[col], ...position } }
		}
		return value
	})
}

// export function moveItemsAroundItem(active, items, cols, original) {
// 	// Get current item from the breakpoint
// 	const activeItem = getItem(active, cols)
// 	const ids = items.map((value) => value.id).filter((value) => value !== activeItem.id)

// 	const els = items.filter((value) => value.id !== activeItem.id)
// 	console.log(1, cols)
// 	// Update items
// 	let newItems = updateItem(items, active, activeItem, cols)

// 	let matrix = makeMatrixFromItemsIgnore(newItems, ids, getRowsCount(newItems, cols), cols)
// 	let tempItems = newItems

// 	// Exclude resolved elements ids in array
// 	let exclude: string[] = []

// 	els.forEach((item) => {
// 		// Find position for element
// 		let position = findFreeSpaceForItem(matrix, item[cols])
// 		// Exclude item
// 		exclude.push(item.id)
// 		console.log(2, cols)

// 		tempItems = updateItem(tempItems, item, position, cols)

// 		// Recreate ids of elements
// 		let getIgnoreItems = ids.filter((value) => exclude.indexOf(value) === -1)

// 		// Update matrix for next iteration
// 		matrix = makeMatrixFromItemsIgnore(
// 			tempItems,
// 			getIgnoreItems,
// 			getRowsCount(tempItems, cols),
// 			cols
// 		)
// 	})

// 	// Return result
// 	return tempItems
// }

export function moveItem(active, items, cols) {
	// Get current item from the breakpoint
	const item = getItem(active, cols)
	// console.log(JSON.stringify(item), JSON.stringify(active), JSON.stringify(cols), 3, cols)

	// Create matrix from the items expect the active
	let matrix = makeMatrixFromItemsIgnore(items, [active.id], getRowsCount(items, cols), cols)
	// Getting the ids of items under active Array<String>
	const closeBlocks = findCloseBlocks(matrix, item)
	// Getting the objects of items under active Array<Object>
	let closeObj = findItemsById(closeBlocks, items)
	// Getting whenever of these items is fixed
	const fixed = closeObj.find((value) => value[cols].fixed)

	// If found fixed, reset the active to its original position
	if (fixed) {
		return {
			items: items
		}
	}

	// Update items
	const nitems = updateItem(items, active, item, cols)

	// Create matrix of items expect close elements
	matrix = makeMatrixFromItemsIgnore(nitems, closeBlocks, getRowsCount(nitems, cols), cols)

	// Create temp vars
	let tempItems = nitems
	let tempCloseBlocks = closeBlocks

	// Exclude resolved elements ids in array
	let exclude: string[] = []

	// Iterate over close elements under active item
	closeObj.forEach((item) => {
		// Find position for element
		let position = findFreeSpaceForItem(matrix, item[cols])
		// Exclude item
		exclude.push(item.id)

		// Assign the position to the element in the column
		tempItems = updateItem(tempItems, item, position, cols)

		// Recreate ids of elements
		let getIgnoreItems = tempCloseBlocks.filter((value) => exclude.indexOf(value) === -1)

		// Update matrix for next iteration
		matrix = makeMatrixFromItemsIgnore(
			tempItems,
			getIgnoreItems,
			getRowsCount(tempItems, cols),
			cols
		)
	})

	// Return result
	return {
		items: tempItems,
		overlap: undefined
	}
}

// Helper function
export function normalize(items, col) {
	let result = items.slice()

	result.forEach((value) => {
		const getItem = value[col]
		if (!getItem.static) {
			result = moveItem(getItem, result, col).items
		}
	})

	return result
}

// Helper function
export function adjust<T>(items: FilledItem<T>[], col) {
	let matrix = makeMatrix(getRowsCount(items, col), col)

	let res: FilledItem<T>[] = []

	items.forEach((item) => {
		let position = findFreeSpaceForItem(matrix, item[col])

		res.push({
			...item,
			[col]: {
				...item[col],
				...position
			}
		})

		matrix = makeMatrixFromItems(res, getRowsCount(res, col), col)
	})

	return res
}

export function getUndefinedItems(items, col, breakpoints) {
	return items
		.map((value) => {
			if (!value[col]) {
				return value.id
			}
		})
		.filter(Boolean)
}

export function getClosestColumn(items, item, col, breakpoints) {
	return breakpoints
		.map(([_, column]) => item[column] && column)
		.filter(Boolean)
		.reduce(function (acc, value) {
			const isLower = Math.abs(value - col) < Math.abs(acc - col)

			return isLower ? value : acc
		})
}

export function specifyUndefinedColumns(items, col, breakpoints) {
	let matrix = makeMatrixFromItems(items, getRowsCount(items, col), col)

	const getUndefinedElements = getUndefinedItems(items, col, breakpoints)

	let newItems = [...items]

	getUndefinedElements.forEach((elementId) => {
		const getElement = items.find((item) => item.id === elementId)

		const closestColumn = getClosestColumn(items, getElement, col, breakpoints)

		const position = findFreeSpaceForItem(matrix, getElement[closestColumn])

		const newItem = {
			...getElement,
			[col]: {
				...getElement[closestColumn],
				...position
			}
		}

		newItems = newItems.map((value) => (value.id === elementId ? newItem : value))

		matrix = makeMatrixFromItems(newItems, getRowsCount(newItems, col), col)
	})
	return newItems
}
