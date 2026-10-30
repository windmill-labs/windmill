import { describe, expect, it } from 'vitest'
import { applyVariableSecretToggle, shouldBlockUnsecretWithoutLoad } from './variableSecretEditor'

describe('shouldBlockUnsecretWithoutLoad', () => {
	it('blocks un-secret when the deployed value was never loaded', () => {
		expect(
			shouldBlockUnsecretWithoutLoad({
				edit: true,
				deployedSecret: true,
				variable: { is_secret: true, value: '' }
			})
		).toBe(true)
	})

	it('allows un-secret after the audited load staged a value', () => {
		expect(
			shouldBlockUnsecretWithoutLoad({
				edit: true,
				deployedSecret: true,
				variable: { is_secret: true, value: 'loaded' }
			})
		).toBe(false)
	})
})

describe('applyVariableSecretToggle', () => {
	const deployedSecret = { is_secret: true, value: '' }

	it('clears a value that was revealed without loading when re-enabling secret', () => {
		expect(
			applyVariableSecretToggle({
				isSecret: true,
				deployedSecret: true,
				draft: { is_secret: true, value: 'leaked' },
				deployedBaseline: deployedSecret
			})
		).toEqual({ value: '', baselineValue: '' })
	})

	it('clears a leaked value when secret is turned off without audit', () => {
		expect(
			applyVariableSecretToggle({
				isSecret: false,
				deployedSecret: true,
				draft: { is_secret: false, value: 'leaked' },
				deployedBaseline: deployedSecret
			})
		).toEqual({ value: '', baselineValue: '' })
	})

	it('keeps a user-edited plaintext when turning secret back on after load', () => {
		const baseline = { is_secret: true, value: 'from-server' }
		expect(
			applyVariableSecretToggle({
				isSecret: true,
				deployedSecret: true,
				draft: { is_secret: true, value: 'user-edit' },
				deployedBaseline: baseline
			})
		).toBeNull()
	})

	it('re-masks after load when the draft still matches the deployed baseline', () => {
		const baseline = { is_secret: true, value: 'from-server' }
		expect(
			applyVariableSecretToggle({
				isSecret: true,
				deployedSecret: true,
				draft: { is_secret: true, value: 'from-server' },
				deployedBaseline: baseline
			})
		).toEqual({ value: '', baselineValue: '' })
	})
})
