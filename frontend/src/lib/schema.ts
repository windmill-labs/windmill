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

/** Args as the JSON payload the JSON editor starts from. Every schema property is spelled out,
 * so an argument with no value yet still shows its name; args the schema does not declare are
 * kept, since what the editor holds replaces the args wholesale on the next keystroke. */
export function argsToJsonPayload(
	schema: Schema | undefined,
	args: Record<string, any> | undefined
): string {
	const nargs = args ?? {}
	// Null prototype: an arg named after an `Object.prototype` member (`constructor`,
	// `toString`) has to be an own key here, or the `in` check below reads it as already
	// present and its value never reaches the payload.
	const payload: Record<string, any> = Object.create(null)
	const props = schema?.properties ?? {}
	// Schema order first, so the payload reads like the form it replaces.
	for (const key of Object.keys(props)) {
		// Own-property read: an arg named after an `Object.prototype` member (`constructor`,
		// `toString`) would otherwise come back as the inherited function, which `JSON.stringify`
		// drops. An arg that is merely absent falls back to the schema default — `args` only
		// carries defaults once a `SchemaForm` has mounted, which the JSON view alone never does.
		payload[key] =
			(Object.prototype.hasOwnProperty.call(nargs, key) ? nargs[key] : props[key]?.default) ?? null
	}
	for (const [key, value] of Object.entries(nargs)) {
		if (!(key in payload)) {
			payload[key] = value
		}
	}
	return JSON.stringify(payload, null, '\t')
}
