import { describe, expect, it } from 'vitest'

import { providerHost } from './iconCache'

/**
 * `providerHost` decides which hostnames are sent to a third-party favicon service.
 * A regression here does not fail visibly — it silently starts disclosing an
 * endpoint — so the cases it withholds are pinned.
 */
describe('providerHost', () => {
	it('returns the host of a public server url', () => {
		expect(providerHost('https://mcp.linear.app/mcp')).toBe('mcp.linear.app')
	})

	it('withholds loopback and bare addresses', () => {
		expect(providerHost('http://localhost:3000/mcp')).toBeUndefined()
		expect(providerHost('http://127.0.0.1:8000/mcp')).toBeUndefined()
		expect(providerHost('http://10.1.2.3/mcp')).toBeUndefined()
		// `new URL()` brackets an IPv6 literal, which has no dot to split on.
		expect(providerHost('http://[::1]:8000/mcp')).toBeUndefined()
	})

	it('withholds intranet names', () => {
		expect(providerHost('https://mcp-box/mcp')).toBeUndefined()
		expect(providerHost('https://mcp.acme.internal/mcp')).toBeUndefined()
		expect(providerHost('https://Tools.Acme.LOCAL/mcp')).toBeUndefined()
	})

	it('returns nothing for a value that is not a url', () => {
		expect(providerHost(undefined)).toBeUndefined()
		expect(providerHost('not a url')).toBeUndefined()
		expect(providerHost({ url: 'https://example.com' })).toBeUndefined()
	})
})
