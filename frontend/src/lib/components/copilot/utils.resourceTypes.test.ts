import { describe, expect, it } from 'vitest'
import { formatResourceTypes } from './utils'

// Hub resource types such as `record` (schema `{}`) or `dbt_profile`
// (`{"type":"object"}`) carry no `properties`, and the schema column is
// nullable, so any workspace can hold a type whose property map is missing.
const resourceTypes = [
	{ name: 'record', schema: {} },
	{ name: 'dbt_profile', schema: { type: 'object' } },
	{ name: 'null_schema', schema: null },
	{ name: 'null_properties', schema: { type: 'object', properties: null } },
	{ name: 'ok', schema: { type: 'object', properties: { host: { type: 'string' } } } }
] as any

describe('formatResourceTypes tolerates resource types without a property map', () => {
	it('emits `any` for typescript and keeps the valid types', () => {
		const out = formatResourceTypes(resourceTypes, 'typescript')
		expect(out).toContain('type Record = any')
		expect(out).toContain('host: string')
	})

	it('emits an indented `pass` body for python', () => {
		const out = formatResourceTypes(resourceTypes, 'python3')
		expect(out).toContain('class record(TypedDict):\n    pass')
		expect(out).toContain('host: str')
	})

	it('emits an empty class body for php', () => {
		const out = formatResourceTypes(resourceTypes, 'php')
		expect(out).toContain('class Record {\n\n}')
		expect(out).toContain('public string $host;')
	})
})
