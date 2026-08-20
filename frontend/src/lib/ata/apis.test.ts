import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('$lib/stores', () => ({
	workspaceStore: {
		subscribe: (run: (v: string) => void) => {
			run('test-workspace')
			return () => {}
		}
	}
}))

/** Fresh module per case: the registry-configured answer is memoized per session. */
async function loadApis(fetchImpl: (url: string) => Promise<Response>) {
	vi.resetModules()
	const requested: string[] = []
	vi.stubGlobal(
		'fetch',
		vi.fn((input: RequestInfo | URL) => {
			const url = String(input)
			requested.push(url)
			return fetchImpl(url)
		})
	)
	return { apis: await import('./apis'), requested }
}

const VERSIONS_FROM_PROXY = '{"tags":{"latest":"1.0.0"},"versions":["1.0.0"]}'
const VERSIONS_FROM_CDN = '{"tags":{"latest":"9.9.9"},"versions":["9.9.9"]}'

function registry(configured: boolean) {
	return async (url: string) => {
		if (url.endsWith('/npm_proxy/config')) {
			return new Response(JSON.stringify({ registry_configured: configured }))
		}
		if (url.includes('/npm_proxy/')) return new Response(VERSIONS_FROM_PROXY)
		if (url.includes('jsdelivr')) return new Response(VERSIONS_FROM_CDN)
		throw new Error(`unexpected request to ${url}`)
	}
}

/** jsdelivr unreachable (blocked egress, DNS failure), backend proxy answering. */
function unreachableCdn(proxyBody: string) {
	return async (url: string) => {
		if (url.endsWith('/npm_proxy/config')) {
			return new Response(JSON.stringify({ registry_configured: false }))
		}
		if (url.includes('jsdelivr')) throw new TypeError('Failed to fetch')
		if (url.includes('/npm_proxy/')) return new Response(proxyBody)
		throw new Error(`unexpected request to ${url}`)
	}
}

describe('ATA source ordering', () => {
	beforeEach(() => vi.unstubAllGlobals())

	it('asks the proxy first when the instance configures a registry', async () => {
		const { apis, requested } = await loadApis(registry(true))

		const versions = await apis.getNPMVersionsForModule('lodash', { usage: 0 })

		expect((versions as { versions: string[] }).versions).toEqual(['1.0.0'])
		expect(requested.some((u) => u.includes('jsdelivr'))).toBe(false)
	})

	it('asks the CDN first when no registry is configured', async () => {
		const { apis, requested } = await loadApis(registry(false))

		const versions = await apis.getNPMVersionsForModule('lodash', { usage: 0 })

		expect((versions as { versions: string[] }).versions).toEqual(['9.9.9'])
		expect(requested.filter((u) => u.includes('/npm_proxy/'))).toEqual([
			'/api/w/test-workspace/npm_proxy/config'
		])
	})

	it('re-asks after a failed config probe rather than pinning the session to the CDN', async () => {
		let configAttempts = 0
		const { apis } = await loadApis(async (url) => {
			if (url.endsWith('/npm_proxy/config')) {
				configAttempts++
				if (configAttempts === 1) throw new TypeError('Failed to fetch')
				return new Response(JSON.stringify({ registry_configured: true }))
			}
			if (url.includes('/npm_proxy/')) return new Response(VERSIONS_FROM_PROXY)
			return new Response(VERSIONS_FROM_CDN)
		})

		const first = await apis.getNPMVersionsForModule('lodash', { usage: 0 })
		const second = await apis.getNPMVersionsForModule('lodash', { usage: 0 })

		expect((first as { versions: string[] }).versions).toEqual(['9.9.9'])
		expect((second as { versions: string[] }).versions).toEqual(['1.0.0'])
	})

	it('falls back to the CDN when the preferred proxy fails', async () => {
		const { apis } = await loadApis(async (url) => {
			if (url.endsWith('/npm_proxy/config')) {
				return new Response(JSON.stringify({ registry_configured: true }))
			}
			if (url.includes('/npm_proxy/')) throw new TypeError('Failed to fetch')
			return new Response(VERSIONS_FROM_CDN)
		})

		const versions = await apis.getNPMVersionsForModule('lodash', { usage: 0 })

		expect((versions as { versions: string[] }).versions).toEqual(['9.9.9'])
	})

	it('falls back to the proxy when the jsdelivr request rejects', async () => {
		const { apis } = await loadApis(unreachableCdn(VERSIONS_FROM_PROXY))

		const versions = await apis.getNPMVersionsForModule('lodash', { usage: 0 })

		expect(versions).not.toBeInstanceOf(Error)
		expect((versions as { versions: string[] }).versions).toEqual(['1.0.0'])
	})

	it('falls back to the proxy for a d.ts when the jsdelivr request rejects', async () => {
		const { apis } = await loadApis(unreachableCdn('declare const x: number'))

		const dts = await apis.getDTSFileForModuleWithVersion('lodash', '1.0.0', '/index.d.ts')

		expect(dts).toBe('declare const x: number')
	})
})
