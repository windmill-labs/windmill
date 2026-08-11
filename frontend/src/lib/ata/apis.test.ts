import { afterEach, describe, expect, it, vi } from 'vitest'
import { workspaceStore } from '$lib/stores'
import { getDTSFileForModuleWithVersion, getNPMVersionsForModule } from './apis'

/** jsdelivr unreachable (blocked egress, DNS failure), backend proxy answering. */
function mockUnreachableJsdelivr(proxyBody: string) {
	return vi.fn(async (input: RequestInfo | URL) => {
		const url = String(input)
		if (url.includes('jsdelivr')) throw new TypeError('Failed to fetch')
		if (url.includes('/npm_proxy/')) return new Response(proxyBody, { status: 200 })
		throw new Error(`unexpected request to ${url}`)
	})
}

describe('ATA backend proxy fallback', () => {
	afterEach(() => vi.unstubAllGlobals())

	it('falls back to the proxy when the jsdelivr request rejects', async () => {
		workspaceStore.set('test-workspace')
		vi.stubGlobal(
			'fetch',
			mockUnreachableJsdelivr('{"tags":{"latest":"1.0.0"},"versions":["1.0.0"]}')
		)

		const versions = await getNPMVersionsForModule('lodash', { usage: 0 })

		expect(versions).not.toBeInstanceOf(Error)
		expect((versions as { versions: string[] }).versions).toEqual(['1.0.0'])
	})

	it('falls back to the proxy for a d.ts when the jsdelivr request rejects', async () => {
		workspaceStore.set('test-workspace')
		vi.stubGlobal('fetch', mockUnreachableJsdelivr('declare const x: number'))

		const dts = await getDTSFileForModuleWithVersion('lodash', '1.0.0', '/index.d.ts')

		expect(dts).toBe('declare const x: number')
	})
})
