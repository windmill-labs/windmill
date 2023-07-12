import type { Schema, SchemaProperty } from '../../common'

import type { ResourceType } from '../../gen'

import { capitalize, toCamel } from '$lib/utils'

function compile(schema: Schema) {
	function rec(x: { [name: string]: SchemaProperty }, root = false) {
		let res = '{ '
		const entries = Object.entries(x)
		if (entries.length == 0) {
			return 'any'
		}
		let i = 0
		for (let [name, prop] of entries) {
			if (prop.type == 'object') {
				res += `${name}: ${rec(prop.properties ?? {})}`
			} else if (prop.type == 'array') {
				res += `${name}: ${prop?.items?.type ?? 'any'}[]`
			} else {
				let typ = prop?.type ?? 'any'
				if (typ == 'integer') {
					typ = 'number'
				}
				res += `${name}: ${typ}`
			}
			i++
			if (i < entries.length) {
				res += ', '
			}
		}
		res += ' }'
		return res
	}
	return rec(schema.properties, true)
}

export function formatResourceTypes(resourceTypes: ResourceType[], camel = false) {
	const result = resourceTypes.map((resourceType) => {
		return `${camel ? toCamel(capitalize(resourceType.name)) : resourceType.name} ${compile(
			resourceType.schema
		)}`
	})

	return result.join(', ')
}
