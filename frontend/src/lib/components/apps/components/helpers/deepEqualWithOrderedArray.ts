import { createCustomEqual } from 'fast-equals'

function compareObjects(obj1: { [key: string]: any }, obj2: { [key: string]: any }): boolean {
	const keys1 = Object.keys(obj1)
	const keys2 = Object.keys(obj2)

	if (keys1.length !== keys2.length) {
		return false
	}

	for (let i = 0; i < keys1.length; i++) {
		const key1 = keys1[i]
		const key2 = keys2[i]

		if (key1 !== key2) {
			return false
		}

		const value1 = obj1[key1]
		const value2 = obj2[key2]

		if (value1 !== value2) {
			return false
		}
	}

	return true
}

const deepEqualWithOrderedArray = createCustomEqual({
	createCustomConfig: (defaultConfig) => ({
		...defaultConfig,
		areObjectsEqual: (a, b, state) => {
			return defaultConfig.areObjectsEqual(a, b, state) && compareObjects(a, b)
		}
	})
})

export default deepEqualWithOrderedArray
