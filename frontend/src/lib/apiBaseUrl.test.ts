import { describe, it, expect } from 'vitest'
import { resolveApiBaseUrl } from './apiBaseUrl.svelte'

describe('resolveApiBaseUrl', () => {
	const origin = 'https://ui.example.com'

	it('uses the configured base', () => {
		expect(resolveApiBaseUrl('https://api.example.com/gw', origin)).toBe(
			'https://api.example.com/gw'
		)
		expect(resolveApiBaseUrl(' https://api.example.com ', origin)).toBe('https://api.example.com')
	})

	it('falls back to the browsing origin when unset', () => {
		for (const unset of [undefined, null, '', '   ', 42, {}]) {
			expect(resolveApiBaseUrl(unset, origin)).toBe(origin)
		}
	})
})
