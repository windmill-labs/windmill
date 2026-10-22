import { beforeEach, describe, expect, it, vi } from 'vitest'

const order: string[] = []

vi.mock('$lib/gen', () => ({
	AppService: {
		getAppLiteByPath: vi.fn(async () => {
			order.push('read the deployed mode')
			return { policy: { execution_mode: 'publisher' } }
		}),
		getAppByPath: vi.fn(async () => ({
			policy: { execution_mode: 'anonymous' },
			draft: { files: {}, runnables: {}, policy: { execution_mode: 'anonymous' } }
		})),
		existsApp: vi.fn(async () => true),
		updateAppRaw: vi.fn(async () => {})
	}
}))
vi.mock('$lib/components/copilot/chat/global/rawAppBundlerBridge', () => ({
	bundleRawAppDraft: vi.fn(async () => {
		order.push('bundle')
		return { js: '', css: '' }
	})
}))
vi.mock('$lib/components/raw_apps/rawAppPolicy', () => ({
	updateRawAppPolicy: vi.fn(async (_runnables: unknown, policy: unknown) => ({ ...(policy as {}) }))
}))

import { AppService } from '$lib/gen'
import { deployRawAppDraft, deployRawAppValue } from './rawAppDeploy'

// Who can open a deployed app is set from the Share dialog. A deploy reads it after bundling,
// which takes seconds, so a Share change made meanwhile is not written back.
describe('raw app deploys', () => {
	beforeEach(() => {
		order.length = 0
		vi.mocked(AppService.updateAppRaw).mockClear()
	})

	const sentMode = () =>
		(vi.mocked(AppService.updateAppRaw).mock.calls[0][0].formData.app as any).policy.execution_mode

	it('deploy a value with the mode deployed once the bundle is built', async () => {
		await deployRawAppValue({
			workspace: 'w',
			path: 'f/ops/app',
			value: { files: {}, runnables: {} },
			policy: { execution_mode: 'anonymous' } as any
		})
		expect(order).toEqual(['bundle', 'read the deployed mode'])
		expect(sentMode()).toBe('publisher')
	})

	it('deploy a draft with the mode deployed once the bundle is built', async () => {
		await deployRawAppDraft('w', 'f/ops/app')
		expect(order).toEqual(['bundle', 'read the deployed mode'])
		expect(sentMode()).toBe('publisher')
	})
})
