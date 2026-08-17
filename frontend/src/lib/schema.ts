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
			// Flow inputs allow names TS cannot use bare, e.g. `user-name`, which
			// would emit an unparseable member. A quoted key means the same thing.
			const name = /^[A-Za-z_$][A-Za-z0-9_$]*$/.test(key) ? key : JSON.stringify(key)
			const prefix = `${name}${isOptional ? '?' : ''}`
			let type: string = 'any'
			if (prop.type === 'string') {
				type = 'string'
			} else if (prop.type === 'number' || prop.type === 'integer') {
				type = 'number'
			} else if (prop.type === 'boolean') {
				type = 'boolean'
			} else if (prop.type === 'array') {
				type = prop.items?.type ?? 'any'
				if (type === 'integer') {
					type = 'number'
				}
				type = `${type}[]`
			} else if (prop.type === 'object' && prop.properties) {
				type = schemaToTsType(prop)
			}

			return `${prefix}: ${type}`
		})
		.join('; ')

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
