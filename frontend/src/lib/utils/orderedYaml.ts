// Kept out of $lib/utils, which every page loads: `yaml` is only needed by the diff views.
import YAML from 'yaml'

function sortObjectKeys(obj: any): any {
	if (obj && typeof obj === 'object' && !Array.isArray(obj)) {
		const sortedObj: any = {}
		Object.keys(obj)
			.sort()
			.forEach((key) => {
				sortedObj[key] = sortObjectKeys(obj[key])
			})
		return sortedObj
	} else if (Array.isArray(obj)) {
		return obj.map((item) => sortObjectKeys(item))
	} else {
		return obj
	}
}

export function orderedYamlStringify(obj: any) {
	const sortedObj = sortObjectKeys(obj)
	return YAML.stringify(sortedObj)
}
