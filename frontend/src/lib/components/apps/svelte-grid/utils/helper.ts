import { adjust, findFreeSpaceForItem } from './item'
import { makeMatrixFromItems } from './matrix'
import { getRowsCount } from './other'

function makeItem(item) {
	const { min = { w: 1, h: 1 }, max } = item
	return {
		fixed: false,
		fullHeight: false,
		min: {
			w: Math.max(1, min.w),
			h: Math.max(1, min.h)
		},
		max: { ...max },
		...item
	}
}

const gridHelp = {
	adjust(items, col) {
		return adjust(items, col)
	},

	item(obj) {
		return makeItem(obj)
	},

	findSpace(item, items, cols) {
		let matrix = makeMatrixFromItems(items, getRowsCount(items, cols), cols)
		let position = findFreeSpaceForItem(matrix, item[cols])
		return position
	}
}

export default gridHelp
