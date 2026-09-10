import { describe, expect, it } from 'vitest'

import { pickMcpIconSrc } from './mcpIcon'

const png = 'data:image/png;base64,iVBORw0KGgo='

/**
 * `src` is chosen by the MCP server and rendered in the user's browser, so what this
 * refuses is a security boundary, not a preference — a regression here fails silently.
 */
describe('pickMcpIconSrc', () => {
	it('takes a data URI of a safe image type', () => {
		expect(pickMcpIconSrc([{ src: png }])).toBe(png)
		expect(pickMcpIconSrc([{ src: 'data:image/webp;base64,UklGRg==' }])).toBe(
			'data:image/webp;base64,UklGRg=='
		)
	})

	// An https src would make the browser fetch from a host the server names, which is
	// the disclosure the favicon lookup was removed for. COEP blocks the response, not
	// the request, so it does not save us.
	it('refuses a fetchable src, whatever the scheme', () => {
		expect(pickMcpIconSrc([{ src: 'https://example.com/logo.png' }])).toBeUndefined()
		expect(pickMcpIconSrc([{ src: 'http://example.com/i.png' }])).toBeUndefined()
		expect(pickMcpIconSrc([{ src: 'javascript:alert(1)' }])).toBeUndefined()
		expect(pickMcpIconSrc([{ src: 'file:///etc/passwd' }])).toBeUndefined()
		expect(pickMcpIconSrc([{ src: 'ftp://example.com/i.png' }])).toBeUndefined()
	})

	// SVG can carry script, and sanitising it is not worth an icon.
	it('refuses SVG', () => {
		expect(pickMcpIconSrc([{ src: 'data:image/svg+xml;base64,PHN2Zz4=' }])).toBeUndefined()
	})

	// `mimeType` is advisory per the spec, so it must not launder the payload.
	it('judges a data URI by what it declares, not by mimeType', () => {
		expect(
			pickMcpIconSrc([{ src: 'data:image/svg+xml;base64,PHN2Zz4=', mimeType: 'image/png' }])
		).toBeUndefined()
	})

	// The chosen src is persisted on a transcript row and re-cloned on every save.
	it('refuses an oversized data URI', () => {
		expect(pickMcpIconSrc([{ src: `data:image/png;base64,${'A'.repeat(9_000)}` }])).toBeUndefined()
	})

	it('takes the first usable entry, skipping ones it refuses', () => {
		expect(pickMcpIconSrc([{ src: 'https://example.com/a.png' }, { src: png }])).toBe(png)
	})

	it('returns nothing for a missing or malformed icons field', () => {
		expect(pickMcpIconSrc(undefined)).toBeUndefined()
		expect(pickMcpIconSrc([])).toBeUndefined()
		expect(pickMcpIconSrc([{ src: 42 }, null, 'nope'])).toBeUndefined()
	})
})
