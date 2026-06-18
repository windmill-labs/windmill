import { describe, expect, it } from 'vitest'

import type { LoadedRunnableSchema } from './runnableSelectorUtils'
import { buildPathRunnableSelection } from './runnableSelectorUtils'

const loadedSchema: LoadedRunnableSchema = {
	summary: 'My flow',
	schema: {
		$schema: 'https://json-schema.org/draft/2020-12/schema',
		type: 'object',
		required: ['string_input'],
		properties: {
			string_input: {
				type: 'string',
				default: ''
			}
		}
	}
}

describe('buildPathRunnableSelection', () => {
	it('keeps the actual schema object and defaults raw-app fields to user mode', () => {
		const selection = buildPathRunnableSelection(
			'u/dev/my_flow',
			'flow',
			loadedSchema,
			false,
			true
		)

		expect(selection.runnable).toMatchObject({
			type: 'path',
			path: 'u/dev/my_flow',
			runType: 'flow',
			schema: loadedSchema.schema,
			name: 'My flow'
		})
		expect(selection.fields.string_input.type).toBe('user')
		expect(selection.fields.string_input.value).toBe('')
	})

	it('preserves static defaults for non-raw-app pickers', () => {
		const selection = buildPathRunnableSelection(
			'u/dev/my_flow',
			'flow',
			loadedSchema,
			false,
			false
		)

		expect(selection.fields.string_input.type).toBe('static')
	})
})
