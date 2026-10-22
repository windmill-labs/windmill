import { describe, expect, it } from 'vitest'
import type { Policy } from '$lib/gen'
import { draftOrDeployedSettings, withDraftPolicySettings } from './appDraftSettings'

describe('withDraftPolicySettings', () => {
	it('keeps a setting the draft turned off, and the whitelist from the deployed policy', () => {
		const deployed = {
			execution_mode: 'anonymous',
			sandbox: true,
			frontend_sdk_scopes: ['resources:read'],
			triggerables_v2: { 'a:rawscript/x': {} }
		} as unknown as Policy
		// Sandbox off drops the key, and the scopes with it.
		const draft = { execution_mode: 'publisher' } as Partial<Policy>
		expect(withDraftPolicySettings(deployed, draft, false)).toEqual({
			execution_mode: 'publisher',
			triggerables_v2: { 'a:rawscript/x': {} }
		})
	})

	it('keeps the deployed access of a deployed app over a draft saved before a Share change', () => {
		const deployed = { execution_mode: 'anonymous', sandbox: true } as unknown as Policy
		const draft = { execution_mode: 'publisher', sandbox: true } as Partial<Policy>
		expect(withDraftPolicySettings(deployed, draft, true)).toEqual({
			execution_mode: 'anonymous',
			sandbox: true
		})
	})
})

describe('draftOrDeployedSettings', () => {
	it('leaves access and custom URL out of a deployed app, and keeps them for a new one', () => {
		const parts = { policy: { execution_mode: 'anonymous' }, customPath: 'invoices' }
		const row = { policy: { execution_mode: 'publisher' } as Policy, custom_path: 'old' }
		expect(draftOrDeployedSettings(parts, row)).toEqual({
			policy: {},
			labels: [],
			custom_path: null
		})
		expect(draftOrDeployedSettings(parts, {})).toEqual({
			policy: { execution_mode: 'anonymous' },
			labels: [],
			custom_path: 'invoices'
		})
	})
})
