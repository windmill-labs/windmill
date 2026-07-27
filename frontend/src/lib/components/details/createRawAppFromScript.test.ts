import { describe, expect, it } from 'vitest'

import { createRawAppFromFlow, createRawAppFromScript } from './createRawAppFromScript'

describe('createRawAppFromScript', () => {
	it('builds a path runnable and a form calling it', () => {
		const app = createRawAppFromScript('u/dev/greet_user', 'Greet a user', {
			type: 'object',
			order: ['name', 'age', 'mode'],
			required: ['name', 'age'],
			properties: {
				name: { type: 'string' },
				age: { type: 'integer', default: 42 },
				mode: { type: 'string', enum: ['fast', 'slow'] }
			}
		})

		expect(app.summary).toBe('Greet a user')
		expect(app.value.runnables).toEqual({
			greet_user: {
				name: 'u/dev/greet_user',
				type: 'path',
				runType: 'script',
				path: 'u/dev/greet_user',
				schema: expect.objectContaining({ type: 'object' }),
				fields: {}
			}
		})

		const appTsx = app.value.files['/App.tsx']
		expect(appTsx).toContain("const [name, setName] = useState('')")
		expect(appTsx).toContain("const [ageText, setAgeText] = useState('42')")
		// Required args are passed unconditionally so they satisfy the non-optional
		// type `genWmillTs` derives from the same schema.
		expect(appTsx).toContain('age: Number(ageText)')
		expect(appTsx).toContain('mode,')
		expect(appTsx).toContain('await backend.greet_user({')
		expect(appTsx).toContain('<option value="slow">slow</option>')
	})

	it('lets an emptied optional number input mean "unset"', () => {
		const app = createRawAppFromScript('u/dev/s', undefined, {
			type: 'object',
			properties: { n: { type: 'number' } }
		})
		expect(app.value.files['/App.tsx']).toContain("n: nText === '' ? undefined : Number(nText)")
	})

	it('renames arguments that would shadow the component locals', () => {
		const app = createRawAppFromFlow('u/dev/f', 'Flow', {
			type: 'object',
			required: ['result'],
			properties: { result: { type: 'string' } }
		})
		const appTsx = app.value.files['/App.tsx']
		expect(appTsx).toContain("const [result_, setResult_] = useState('')")
		expect(appTsx).toContain('result: result_')
		expect(app.value.runnables['f'].runType).toBe('flow')
	})

	it('keeps every generated binding unique and syntactically valid', () => {
		const app = createRawAppFromScript('u/dev/s', undefined, {
			type: 'object',
			order: ['foo', 'setFoo', 'class'],
			properties: {
				foo: { type: 'string' },
				setFoo: { type: 'string' },
				class: { type: 'string' }
			}
		})
		const appTsx = app.value.files['/App.tsx']
		expect(appTsx).toContain("const [foo, setFoo] = useState('')")
		expect(appTsx).toContain("const [setFoo_, setSetFoo_] = useState('')")
		expect(appTsx).toContain("const [class_, setClass_] = useState('')")

		const declared = [...appTsx.matchAll(/const \[(\w+), (\w+)\]/g)].flatMap((m) => [m[1], m[2]])
		expect(new Set(declared).size).toBe(declared.length)
	})

	it('does not let an argument shadow a global the generated body calls', () => {
		const app = createRawAppFromScript('u/dev/s', undefined, {
			type: 'object',
			order: ['JSON', 'eval'],
			properties: { JSON: { type: 'string' }, eval: { type: 'string' } }
		})
		const appTsx = app.value.files['/App.tsx']
		expect(appTsx).toContain("const [JSON_, setJSON_] = useState('')")
		expect(appTsx).toContain("const [eval_, setEval_] = useState('')")
		// The result panel must still reach the real global.
		expect(appTsx).toContain('JSON.stringify(result, null, 2)')
	})

	it('keeps value and label apart for labeled enums', () => {
		const app = createRawAppFromScript('u/dev/s', undefined, {
			type: 'object',
			properties: {
				mode: {
					type: 'string',
					enum: [
						{ value: 'fast', label: 'Fast (cached)' },
						{ value: 'slow', label: 'Slow (fresh)' }
					]
				}
			}
		})
		const appTsx = app.value.files['/App.tsx']
		expect(appTsx).toContain('<option value="fast">Fast (cached)</option>')
		expect(appTsx).toContain('<option value="slow">Slow (fresh)</option>')
		expect(appTsx).toContain("const [mode, setMode] = useState('fast')")
		expect(appTsx).not.toContain('[object Object]')
	})
})
