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

	it('keeps the undefined sentinel reachable when an argument is named undefined', () => {
		const app = createRawAppFromScript('u/dev/s', undefined, {
			type: 'object',
			order: ['undefined', 'n'],
			properties: { undefined: { type: 'string' }, n: { type: 'number' } }
		})
		const appTsx = app.value.files['/App.tsx']
		expect(appTsx).toContain("const [undefined_, setUndefined_] = useState('')")
		expect(appTsx).toContain('undefined: undefined_')
		// The omitted-optional sentinel and the render guards must still be the
		// real global, not the field's value.
		expect(appTsx).toContain("nText === '' ? undefined : Number(nText)")
		expect(appTsx).toContain('{result !== undefined &&')
		expect(appTsx).toContain('{error !== undefined &&')
	})

	it('masks password arguments and declares them sensitive on the runnable', () => {
		const app = createRawAppFromScript('u/dev/s', undefined, {
			type: 'object',
			required: ['token'],
			properties: { token: { type: 'string', password: true }, plain: { type: 'string' } }
		})
		expect(app.value.files['/App.tsx']).toContain('type="password"')
		// The policy's `sensitive_inputs` is derived from these fields, so without
		// them the secret is stored in the job args in the clear.
		expect(app.value.runnables['s'].fields).toEqual({
			token: { type: 'user', value: undefined, sensitive: true }
		})
	})

	it('passes a __proto__ argument as an own property', () => {
		const app = createRawAppFromScript('u/dev/s', undefined, {
			type: 'object',
			required: ['__proto__'],
			// Computed key: a plain `__proto__:` here would set this literal's
			// prototype and the property would not exist at all.
			properties: { ['__proto__']: { type: 'number' } }
		})
		// `__proto__: value` in an object literal sets the prototype instead of
		// creating the argument, so it has to be a computed key.
		expect(app.value.files['/App.tsx']).toContain("['__proto__']: Number(__proto__Text)")
	})

	it('omits blank optional text instead of sending an empty string', () => {
		const app = createRawAppFromScript('u/dev/s', undefined, {
			type: 'object',
			required: ['req'],
			properties: { req: { type: 'string' }, opt: { type: 'string' } }
		})
		const appTsx = app.value.files['/App.tsx']
		expect(appTsx).toContain("opt: opt === '' ? undefined : opt")
		expect(appTsx).toContain('req,')
		// The required marker has to be enforced, not just drawn.
		expect(appTsx).toMatch(/type="text"\n\t+required\n\t+value=\{req\}/)
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
