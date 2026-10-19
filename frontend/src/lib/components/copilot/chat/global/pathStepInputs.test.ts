import { describe, expect, it } from 'vitest'
import type { FlowModule } from '$lib/gen'
import { collectPathSteps, missingStepInputs } from './pathStepInputs'

const call = (id: string, path: string, inputs: string[]): FlowModule => ({
	id,
	value: {
		type: 'script',
		path,
		input_transforms: Object.fromEntries(inputs.map((n) => [n, { type: 'static', value: '' }]))
	}
})

const EMAIL_SCHEMA = {
	required: ['subject', 'body', 'cc'],
	properties: { subject: {}, body: {}, cc: { default: [] } }
}
const schemaOf = (step: { path: string }) =>
	step.path === 'f/finance/email_finance' ? EMAIL_SCHEMA : undefined

describe('missingStepInputs', () => {
	it('reports required inputs a nested step leaves out, and only new gaps', () => {
		const flow = {
			modules: [
				{
					id: 'loop',
					value: {
						type: 'forloopflow',
						iterator: { type: 'javascript', expr: '[]' },
						skip_failures: false,
						modules: [call('c', 'f/finance/email_finance', ['subject'])]
					}
				},
				call('d', 'f/finance/unknown', [])
			]
		} as unknown as { modules: FlowModule[] }
		const next = collectPathSteps(flow)
		expect(missingStepInputs(next, [], schemaOf)).toEqual([
			{ id: 'c', path: 'f/finance/email_finance', missing: ['body'] }
		])
		// The same gap was already there before the edit: not this write's to fix.
		expect(missingStepInputs(next, next, schemaOf)).toEqual([])
	})

	it('leaves out the preprocessor, whose inputs the worker ignores', () => {
		const flow = {
			modules: [],
			preprocessor_module: call('preprocessor', 'f/finance/email_finance', [])
		}
		expect(collectPathSteps(flow)).toEqual([])
	})
})
