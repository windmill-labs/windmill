import { describe, expect, it } from 'vitest'
import { conformArgsToSchema } from './job_args'

describe('conformArgsToSchema', () => {
	it('drops what the schema does not declare, prototype names included', () => {
		const { args, droppedKeys } = conformArgsToSchema(
			JSON.parse('{"keep":1,"force":true,"constructor":"x","toString":"y","__proto__":{"p":1}}'),
			{ properties: { keep: { type: 'number' } } }
		)
		expect(args).toEqual({ keep: 1 })
		expect(Object.getPrototypeOf(args)).toBe(Object.prototype)
		expect(droppedKeys.sort()).toEqual(['__proto__', 'constructor', 'force', 'toString'])
	})
})
