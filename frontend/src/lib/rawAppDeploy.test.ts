import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('$lib/gen', () => ({
	AppService: {
		getAppByPath: vi.fn(async () => ({
			policy: { execution_mode: 'anonymous' },
			draft: { files: {}, runnables: {}, policy: { execution_mode: 'anonymous' } }
		})),
		existsApp: vi.fn(async () => true),
		updateAppRaw: vi.fn(async () => {})
	}
}))
vi.mock('$lib/components/copilot/chat/global/rawAppBundlerBridge', () => ({
	bundleRawAppDraft: vi.fn(async () => ({ js: '', css: '' }))
}))
vi.mock('$lib/components/raw_apps/rawAppPolicy', () => ({
	updateRawAppPolicy: vi.fn(async (_runnables: unknown, policy: unknown) => ({ ...(policy as {}) }))
}))

import { AppService } from '$lib/gen'
import { deployRawAppDraft, deployRawAppValue } from './rawAppDeploy'

// Who can open a deployed app is set from the Share dialog. A deploy stating a mode would put back
// the one it was given, so an update states none and the server keeps the deployed one.
describe('raw app deploys', () => {
	beforeEach(() => {
		vi.mocked(AppService.updateAppRaw).mockClear()
	})

	const sentPolicy = () =>
		(vi.mocked(AppService.updateAppRaw).mock.calls[0][0].formData.app as any).policy

	it('deploy a value without stating a mode', async () => {
		await deployRawAppValue({
			workspace: 'w',
			path: 'f/ops/app',
			value: { files: {}, runnables: {} },
			policy: { execution_mode: 'anonymous' } as any
		})
		expect(sentPolicy()).not.toHaveProperty('execution_mode')
	})

	it('deploy a draft without stating a mode', async () => {
		await deployRawAppDraft('w', 'f/ops/app')
		expect(sentPolicy()).not.toHaveProperty('execution_mode')
	})
})
