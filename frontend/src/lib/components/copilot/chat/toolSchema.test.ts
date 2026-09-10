import { describe, expect, it } from 'vitest'

import { normalizeToolParameterSchema } from './toolSchema'

describe('normalizeToolParameterSchema', () => {
	// `required` is `uniqueItems` at every subschema, and a provider rejects the whole
	// request over a repeat — which for a tool built from a third party's schema means
	// every send fails until that tool goes away. `safeInputSchema` only de-dupes the
	// root, so the nested case is this function's alone.
	it('de-dupes required at every depth', () => {
		const schema: Record<string, any> = {
			type: 'object',
			properties: {
				user: {
					type: 'object',
					properties: { id: { type: 'string' } },
					required: ['id', 'id']
				},
				tags: { type: 'array', items: { type: 'object', required: ['name', 'name'] } }
			},
			anyOf: [{ type: 'object', required: ['a', 'a'] }],
			required: ['user', 'user']
		}

		normalizeToolParameterSchema(schema)

		expect(schema.required).toEqual(['user'])
		expect(schema.properties.user.required).toEqual(['id'])
		expect(schema.properties.tags.items.required).toEqual(['name'])
		expect(schema.anyOf[0].required).toEqual(['a'])
	})
})
