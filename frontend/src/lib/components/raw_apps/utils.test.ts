import { describe, expect, it } from 'vitest'

import { genWmillTs, normalizeRawAppRunnables, type Runnable } from './utils'

const flowSchema = {
	$schema: 'https://json-schema.org/draft/2020-12/schema',
	type: 'object',
	required: ['string_input'],
	properties: {
		string_input: {
			type: 'string',
			default: ''
		},
		count: {
			type: 'integer'
		}
	}
} as const

function buildLegacyFlowRunnable(
	fieldValue: string,
	fieldType: 'static' | 'user' = 'static'
): Runnable {
	return {
		type: 'path',
		runType: 'flow',
		path: 'u/dev/my_flow',
		name: 'My flow',
		schema: {
			summary: 'My flow',
			schema: flowSchema
		},
		fields: {
			string_input: {
				type: fieldType,
				value: fieldValue,
				fieldType: 'text'
			}
		}
	}
}

describe('normalizeRawAppRunnables', () => {
	it('unwraps legacy flow schemas and converts generated static defaults to user fields', () => {
		const runnables = normalizeRawAppRunnables({
			myflow: buildLegacyFlowRunnable('')
		})

		expect(runnables.myflow).toMatchObject({
			type: 'path',
			runType: 'flow',
			schema: flowSchema
		})
		expect(runnables.myflow?.fields?.string_input.type).toBe('user')
	})

	it('preserves customized static fields', () => {
		const runnables = normalizeRawAppRunnables({
			myflow: buildLegacyFlowRunnable('preset')
		})

		expect(runnables.myflow?.fields?.string_input.type).toBe('static')
		expect((runnables.myflow?.fields?.string_input as { value?: string } | undefined)?.value).toBe(
			'preset'
		)
	})
})

describe('genWmillTs', () => {
	it('generates the correct flow args type after normalization', () => {
		const dts = genWmillTs(
			normalizeRawAppRunnables({
				myflow: buildLegacyFlowRunnable('')
			})
		)

		expect(dts).toContain(
			'myflow: (args: { string_input: string; count?: number }) => Promise<any>;'
		)
	})
})
