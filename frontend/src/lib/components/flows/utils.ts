function filterByKey(obj: Object, key: string): Object {
	if (Object(obj) !== obj) {
		return obj
	} else if (Array.isArray(obj)) {
		debugger
		return obj.map((o) => filterByKey(o, key))
	} else {
		return Object.fromEntries(
			Object.entries(obj)
				.filter(([k, v]) => !k.includes(key))
				.map(([k, v]) => [k, filterByKey(v, key)])
		)
	}
}

function diff(target: Object, source: Object): Object {
	if (Array.isArray(target)) {
		return target
	}

	const result = {}

	Object.keys(target).forEach((key: string) => {
		if (typeof source[key] === 'object') {
			const difference = diff(target[key], source[key])

			if (Object.keys(difference).length > 0) {
				result[key] = difference
			}
		} else if (source[key] !== target[key]) {
			result[key] = target[key]
		}
	})

	return result
}

export function keepByKey(json: Object, key: string): Object {
	return diff(json, filterByKey(json, key))
}

export function getTypeAsString(arg: any): string {
	if (arg === null) {
		return 'null'
	}
	return typeof arg
}

export function formatValue(arg: any) {
	if (getTypeAsString(arg) === 'string') {
		return `"${arg}"`
	}
	return arg
}
