import type { Schema, SchemaProperty } from './common'

export function schemaToTsType(schema: Schema | SchemaProperty): string {
	const schemaProperties = schema.properties
	const schemaRequired = schema.required
	if (!schema || !schemaProperties) {
		return 'any'
	}
	const propKeys = Object.keys(schemaProperties)

	const types = propKeys
		.map((key: string) => {
			const prop = schemaProperties[key]
			const isOptional = !schemaRequired?.includes(key)
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
			} else if (prop.type === 'object' && prop.properties) {
				type = schemaToTsType(prop)
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

export function mergeObjectSchemasWithUnion(schemas: Schema[]): Schema {
	const merged: Schema = {
		$schema: 'https://json-schema.org/draft/2020-12/schema',
		required: [],
		properties: {},
		type: 'object'
	}

	for (const s of schemas) {
		for (const [propertyName, property] of Object.entries(s.properties)) {
			if (!(propertyName in merged.properties)) {
				merged.properties[propertyName] = property
			} else {
				// TODO : Union type and recursion
			}
		}
	}

	for (const propertyName of Object.keys(merged.properties)) {
		if (schemas.every((s) => s.required.includes(propertyName))) {
			merged.required.push(propertyName)
		}
	}

	return merged
}
