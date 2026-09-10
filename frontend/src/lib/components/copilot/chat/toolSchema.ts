/**
 * Recursively normalizes JSON Schema quirks that specific providers reject.
 *
 * Its own module rather than part of `shared`: a tool schema can come from a third
 * party (an MCP server's `inputSchema`), so this runs on paths that have no business
 * pulling in the chat's stores, and its tests have no business mocking them.
 */
export function normalizeToolParameterSchema(schema: Record<string, any> | undefined): void {
	if (!schema || typeof schema !== 'object') {
		return
	}

	// Remove format if it's null or empty string
	if (schema.format === null || schema.format === '') {
		delete schema.format
	}

	// `required` is `uniqueItems` at every subschema, and a provider rejects the whole
	// request over a repeat — which for a tool built from a third party's schema means
	// every send fails until that tool goes away.
	if (Array.isArray(schema.required)) {
		schema.required = [...new Set(schema.required.filter((n: unknown) => typeof n === 'string'))]
	}

	// Recurse into properties
	if (schema.properties && typeof schema.properties === 'object') {
		for (const key of Object.keys(schema.properties)) {
			normalizeToolParameterSchema(schema.properties[key])
		}
	}

	// Recurse into items (for arrays)
	if (schema.items) {
		if (Array.isArray(schema.items)) {
			for (const item of schema.items) {
				normalizeToolParameterSchema(item)
			}
		} else {
			normalizeToolParameterSchema(schema.items)
		}
	}

	// Recurse into additionalProperties if it's an object schema
	if (schema.additionalProperties && typeof schema.additionalProperties === 'object') {
		normalizeToolParameterSchema(schema.additionalProperties)
	}

	// Recurse into allOf, anyOf, oneOf
	for (const key of ['allOf', 'anyOf', 'oneOf']) {
		if (Array.isArray(schema[key])) {
			for (const subSchema of schema[key]) {
				normalizeToolParameterSchema(subSchema)
			}
		}
	}
}
