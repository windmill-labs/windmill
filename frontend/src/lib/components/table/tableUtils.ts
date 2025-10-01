import { Parser } from '@json2csv/plainjs'

export function isLink(value: string) {
	return value?.startsWith('http://') || value?.startsWith('https://')
}

export function isEmail(value: string) {
	return value?.includes('@')
}

export function computeStructuredObjectsAndHeaders(objects: Array<Record<string, any>>): [
	string[],
	{
		_id: number
		rowData: Record<string, any>
	}[]
] {
	if (Array.isArray(objects)) {
		let nextId = 1

		let hds: string[] = []
		let objs = objects.map((obj) => {
			let rowData = obj && typeof obj == 'object' ? obj : {}
			if (Array.isArray(rowData)) {
				rowData = Object.fromEntries(rowData.map((x, i) => ['col' + i, x]))
			}
			let ks = Object.keys(rowData)
			ks.forEach((x) => {
				if (!hds.includes(x)) {
					hds.push(x)
				}
			})
			return {
				_id: nextId++,
				rowData
			}
		})
		return [hds, objs]
	} else {
		return [[], []]
	}
}

export function convertJsonToCsv(arr: Array<Record<string, any>>): string {
	try {
		const parser = new Parser({})
		const csv = parser.parse(arr)
		return csv
	} catch (err) {
		throw new Error('An error occurred when generating CSV:' + err)
	}
}
