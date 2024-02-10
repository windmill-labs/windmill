import type { Schema, SchemaProperty } from '../../common'

import type { ResourceType } from '../../gen'

import { capitalize, toCamel } from '$lib/utils'
import YAML from 'yaml'

function compile(schema: Schema) {
	function rec(x: { [name: string]: SchemaProperty }, root = false) {
		let res = '{\n'
		const entries = Object.entries(x)
		if (entries.length == 0) {
			return 'any'
		}
		let i = 0
		for (let [name, prop] of entries) {
			if (prop.type == 'object') {
				res += `  ${name}: ${rec(prop.properties ?? {})}`
			} else if (prop.type == 'array') {
				res += `  ${name}: ${prop?.items?.type ?? 'any'}[]`
			} else {
				let typ = prop?.type ?? 'any'
				if (typ == 'integer') {
					typ = 'number'
				}
				res += `  ${name}: ${typ}`
			}
			i++
			if (i < entries.length) {
				res += ',\n'
			}
		}
		res += '\n}'
		return res
	}

	return rec(schema.properties, true)
}

export function pythonCompile(schema: Schema) {
	let res = ''
	const entries = Object.entries(schema.properties)
	if (entries.length === 0) {
		return 'dict'
	}
	let i = 0
	for (let [name, prop] of entries) {
		let typ = 'dict'
		if (prop.type === 'array') {
			typ = 'list'
		} else if (prop.type === 'string') {
			typ = 'str'
		} else if (prop.type === 'number') {
			typ = 'float'
		} else if (prop.type === 'integer') {
			typ = 'int'
		} else if (prop.type === 'boolean') {
			typ = 'bool'
		}
		res += `    ${name}: ${typ}`
		i++
		if (i < entries.length) {
			res += '\n'
		}
	}
	return res
}

export function formatResourceTypes(resourceTypes: ResourceType[], lang: 'python3' | 'typescript') {
	if (lang === 'python3') {
		const result = resourceTypes.map((resourceType) => {
			return `class ${resourceType.name}(TypedDict):\n${pythonCompile(resourceType.schema)}`
		})
		return '\n' + result.join('\n\n')
	} else {
		const result = resourceTypes
			.filter(
				(resourceType) => Boolean(resourceType.schema) && typeof resourceType.schema === 'object'
			)
			.map((resourceType) => {
				return `type ${toCamel(capitalize(resourceType.name))} = ${compile(resourceType.schema)}`
			})
		return '\n' + result.join('\n\n')
	}
}

export function yamlStringifyExceptKeys(obj: any, keys: string[]) {
	return YAML.stringify(obj, (key, val) => {
		if (keys.includes(key)) {
			return undefined
		} else {
			return val
		}
	})
}
