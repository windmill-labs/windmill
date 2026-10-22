import { describe, expect, it } from 'vitest'
import type { Policy } from '$lib/gen'
import { withDraftPolicySettings } from './appDraftSettings'

describe('withDraftPolicySettings', () => {
	it('keeps a setting the draft turned off, and the undrafted fields from the deployed policy', () => {
		const deployed = {
			execution_mode: 'anonymous',
			sandbox: true,
			frontend_sdk_scopes: ['resources:read'],
			triggerables_v2: { 'a:rawscript/x': {} }
		} as unknown as Policy
		// Sandbox off drops the key, and the scopes with it; a stale drafted access mode is ignored.
		const draft = { execution_mode: 'publisher' } as Partial<Policy>
		expect(withDraftPolicySettings(deployed, draft)).toEqual({
			execution_mode: 'anonymous',
			triggerables_v2: { 'a:rawscript/x': {} }
		})
	})
})
