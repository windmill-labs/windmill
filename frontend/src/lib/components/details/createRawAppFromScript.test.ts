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
})
