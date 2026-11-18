import { describe, expect, it, vi } from 'vitest'

vi.mock('$lib/gen', () => ({
	AppService: {
		updateApp: vi.fn(async () => ({})),
		getAppLiteByPath: vi.fn(async () => ({ policy: { execution_mode: 'anonymous' } }))
	}
}))

import { AppService } from '$lib/gen'
import { modeChangeRefusal, updateDeployedAppAccess } from './appAccess'

describe('updateDeployedAppAccess', () => {
	// A policy is written whole, so sending one would put back whatever a deploy wrote since it
	// was read. The mode alone is applied by the server under the deploy's row lock.
	it('sends the mode alone, never a policy', async () => {
		await updateDeployedAppAccess('w', 'f/ops/app', { execution_mode: 'anonymous' })
		expect(vi.mocked(AppService.updateApp).mock.calls[0][0].requestBody).toEqual({
			execution_mode: 'anonymous',
			skip_draft_deletion: true
		})
	})
})

// The server's mode-only checks: a mismatch shows enabled buttons that answer with a 403.
describe('modeChangeRefusal', () => {
	const bob = { on_behalf_of: 'u/bob', sandbox: true }
	const mine = { on_behalf_of: 'u/me', sandbox: true }
	const as = (o: { canPreserveRunAs?: boolean; composeOnly?: boolean }) => ({
		username: 'me',
		canPreserveRunAs: false,
		composeOnly: false,
		...o
	})

	it('refuses an app run as someone else to a caller who may not keep that user', () => {
		expect(modeChangeRefusal(bob, as({}))).toBe('run_as')
		expect(modeChangeRefusal(bob, as({ canPreserveRunAs: true }))).toBeUndefined()
		expect(modeChangeRefusal(mine, as({}))).toBeUndefined()
	})

	it('refuses a builder on an app run as someone else, or not sandboxed', () => {
		expect(modeChangeRefusal(bob, as({ composeOnly: true }))).toBe('run_as')
		// Their deploy keeps u/bob, so deploying alone does not lift this one.
		expect(modeChangeRefusal(bob, as({ composeOnly: true, canPreserveRunAs: true }))).toBe(
			'builder_run_as'
		)
		expect(modeChangeRefusal({ ...mine, sandbox: undefined }, as({ composeOnly: true }))).toBe(
			'unsandboxed'
		)
		expect(modeChangeRefusal(mine, as({ composeOnly: true }))).toBeUndefined()
	})
})
