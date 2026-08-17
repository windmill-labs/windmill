import { afterEach, describe, expect, it, vi } from 'vitest'
import { JobService } from '$lib/gen'
import { makeLaunch } from './cascadeRun'

afterEach(() => vi.restoreAllMocks())

// The client orchestrates the cascade closure, so every launch must carry
// `_wmill_skip_asset_dispatch: true`. A caller arg (e.g. the run form for the
// root node) must NOT be able to override that guard back to false and let the
// backend also dispatch deployed subscribers.
describe('makeLaunch dispatch guard', () => {
	it('local preview: a caller `_wmill_skip_asset_dispatch: false` cannot re-enable dispatch', async () => {
		const spy = vi.spyOn(JobService, 'runScriptPreview').mockResolvedValue('job-1' as any)
		const launch = makeLaunch({
			workspace: 'w',
			resolveLocal: () => ({ content: 'x', language: 'bun' as any }),
			argsFor: () => ({ _wmill_skip_asset_dispatch: false, foo: 1 })
		})
		await launch('f/x/root')
		const body = (spy.mock.calls[0][0] as any).requestBody
		expect(body.args._wmill_skip_asset_dispatch).toBe(true) // guard wins
		expect(body.args.foo).toBe(1) // other caller args preserved
	})

	it('deployed by-path: a caller `_wmill_skip_asset_dispatch: false` cannot re-enable dispatch', async () => {
		const spy = vi.spyOn(JobService, 'runScriptByPath').mockResolvedValue('job-2' as any)
		const launch = makeLaunch({
			workspace: 'w',
			argsFor: () => ({ _wmill_skip_asset_dispatch: false })
		})
		await launch('f/x/deployed')
		const body = (spy.mock.calls[0][0] as any).requestBody
		expect(body._wmill_skip_asset_dispatch).toBe(true)
	})
})
