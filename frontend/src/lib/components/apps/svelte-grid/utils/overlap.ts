import type { FilledItem } from '../types'

export function segmentsOverlap(x1: number, length1: number, x2: number, length2: number): boolean {
	const end1: number = x1 + length1
	const end2: number = x2 + length2

	return Math.max(x1, x2) < Math.min(end1, end2)
}

export function updateComponentVisibility<T>(
	detail: FilledItem<T>,
	items: FilledItem<T>[],
	getComputedCols: number
): boolean {
	const c = items.find((x) => x.id === detail.id)

	if (!c) return false

	return items.some((item) => {
		if (item?.data?.['fullHeight'] !== true || item.id === detail.id) {
			return false
		}

		if (item[getComputedCols]) {
			let { x, y, w } = item[getComputedCols]
			let { x: x1, y: y1, w: w1 } = c[getComputedCols]

			return y < y1 && segmentsOverlap(x, w, x1, w1)
		}
		return false
	})
}
