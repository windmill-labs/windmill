import type { FilledItem } from '../types'

export const makeMatrix: (w: number, h: number) => any[][] = (rows, cols) =>
	Array.from(Array(rows), () => new Array(cols)) // make 2d array

export function makeMatrixFromItems<T>(
	items: FilledItem<T>[],
	row: number,
	col: number
): FilledItem<T>[][] {
	let matrix = makeMatrix(row, col)

	for (var i = 0; i < items.length; i++) {
		const value = items[i][col]
		if (value) {
			const { x, y, h } = value
			const id = items[i].id
			const w = Math.min(col, value.w)

			for (var j = y; j < y + h; j++) {
				const row = matrix[j]
				for (var k = x; k < x + w; k++) {
					row[k] = { ...value, id }
				}
			}
		}
	}
	return matrix
}

export function findCloseBlocks<T>(matrix: FilledItem<T>[][], curObject) {
	const { h, x, y } = curObject

	const w = Math.min(matrix[0].length, curObject.w)
	const tempR = matrix.slice(y, y + h)

	let result: string[] = []
	for (var i = 0; i < tempR.length; i++) {
		let tempA = tempR[i].slice(x, x + w)
		result = [
			...result,
			...tempA.map((val) => val?.id).filter((id) => id !== undefined && id !== curObject.id)
		]
	}

	return [...new Set(result)]
}

export function makeMatrixFromItemsIgnore<T>(
	items: FilledItem<T>[],
	ignoreList: string[],
	row: number,
	col: number
) {
	let matrix = makeMatrix(row, col)
	for (var i = 0; i < items.length; i++) {
		const value = items[i][col]
		const id = items[i].id
		const { x, y, h } = value
		const w = Math.min(col, value.w)

		if (ignoreList.indexOf(id) === -1) {
			for (var j = y; j < y + h; j++) {
				const row = matrix[j]
				if (row) {
					for (var k = x; k < x + w; k++) {
						row[k] = { ...value, id }
					}
				}
			}
		}
	}
	return matrix
}

export function findItemsById<T>(closeBlocks: string[], items: FilledItem<T>[]) {
	return items.filter((value) => closeBlocks.indexOf(value.id) !== -1)
}
