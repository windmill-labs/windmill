import { createCustomEqual, deepEqual } from 'fast-equals'

function compareObjectsKeysOrder(
	obj1: { [key: string]: any },
	obj2: { [key: string]: any }
): boolean {
	return deepEqual(Object.keys(obj1), Object.keys(obj2))
}

const deepEqualWithOrderedArray = createCustomEqual({
	createCustomConfig: (defaultConfig) => ({
		...defaultConfig,
		areObjectsEqual: (a, b, state) => {
			return defaultConfig.areObjectsEqual(a, b, state) && compareObjectsKeysOrder(a, b)
		}
	})
})

export default deepEqualWithOrderedArray
