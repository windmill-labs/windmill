import { afterEach, describe, expect, it, vi } from 'vitest'
import { isChromiumBrowser } from './utils'

// Gates Chromium-only capabilities (DOM screenshot capture). A wrong "true" on
// Gecko/WebKit re-enables the capture path whose spacing/wrapping artifacts the
// gate exists to avoid; a wrong "false" on Chromium silently drops the tool.
describe('isChromiumBrowser', () => {
	afterEach(() => {
		vi.unstubAllGlobals()
	})

	it('detects Chromium via userAgentData brands (any Chromium-based browser)', () => {
		vi.stubGlobal('navigator', {
			userAgentData: {
				brands: [
					{ brand: 'Not-A.Brand', version: '99' },
					{ brand: 'Chromium', version: '138' },
					{ brand: 'Microsoft Edge', version: '138' }
				]
			},
			userAgent: 'anything'
		})
		expect(isChromiumBrowser()).toBe(true)
	})

	it('falls back to the UA string when userAgentData is absent (older Chromium)', () => {
		vi.stubGlobal('navigator', {
			userAgent:
				'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/109.0.0.0 Safari/537.36'
		})
		expect(isChromiumBrowser()).toBe(true)
	})

	it('is false on Firefox ("Chrome/" never appears in a Gecko UA)', () => {
		vi.stubGlobal('navigator', {
			userAgent: 'Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0'
		})
		expect(isChromiumBrowser()).toBe(false)
	})

	it('is false on Safari', () => {
		vi.stubGlobal('navigator', {
			userAgent:
				'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Safari/605.1.15'
		})
		expect(isChromiumBrowser()).toBe(false)
	})

	it('is false with no navigator at all (SSR)', () => {
		vi.stubGlobal('navigator', undefined)
		expect(isChromiumBrowser()).toBe(false)
	})
})
