import { describe, expect, it } from 'vitest'
import { newFlowInputOf } from './stepInputFlowInputs'

describe('newFlowInputOf', () => {
	const pickable = { flow_input: { name: 'x' }, priorIds: {} } as any

	it('names a flow input the flow does not have yet', () => {
		expect(newFlowInputOf('flow_input.email', pickable)).toBe('email')
		expect(newFlowInputOf('flow_input.email.domain', pickable)).toBe('email')
	})

	it('ignores existing flow inputs and other expressions', () => {
		expect(newFlowInputOf('flow_input.name', pickable)).toBeUndefined()
		expect(newFlowInputOf('results.a.email', pickable)).toBeUndefined()
		expect(newFlowInputOf('"static"', pickable)).toBeUndefined()
		expect(newFlowInputOf('flow_input.email', undefined)).toBeUndefined()
	})
})
