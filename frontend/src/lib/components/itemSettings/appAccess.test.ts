import { describe, expect, it, vi } from 'vitest'

vi.mock('$lib/gen', () => ({
	AppService: {
		updateApp: vi.fn(async () => ({})),
		getAppLiteByPath: vi.fn(async () => ({ policy: { execution_mode: 'anonymous' } }))
	}
}))

import { AppService } from '$lib/gen'
import { updateDeployedAppAccess } from './appAccess'

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
