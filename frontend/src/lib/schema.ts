import type { Schema } from './common'

export function schemaToTsType(schema: Schema): string {
	if (!schema || !schema.properties) {
		return 'any'
	}
	const propKeys = Object.keys(schema.properties)

	const types = propKeys
		.map((key: string) => {
			const prop = schema.properties[key]
			const isOptional = !schema.required.includes(key)
			const prefix = `${key}${isOptional ? '?' : ''}`
			let type: string = 'any'
			if (prop.type === 'string') {
				type = 'string'
			} else if (prop.type === 'number' || prop.type === 'integer') {
				type = 'number'
			} else if (prop.type === 'boolean') {
				type = 'boolean'
			} else if (prop.type === 'array') {
				let type = prop.items?.type ?? 'any'
				if (type === 'integer') {
					type = 'number'
				}
				type = `${type}[]`
			}

			return `${prefix}: ${type}`
		})
		.join(';')

	return `{ ${types} }`
}

export function schemaToObject(schema: Schema, args: Record<string, any>): Object {
	const object = {}
	if (!schema || !schema.properties) {
		return object
	}
	const propKeys = Object.keys(schema.properties)

	propKeys.forEach((key: string) => {
		object[key] = args[key] ?? null
	})
	return object
}
