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

	it('refuses schemes the spec calls unsafe', () => {
		expect(pickMcpIconSrc([{ src: 'javascript:alert(1)' }])).toBeUndefined()
		expect(pickMcpIconSrc([{ src: 'file:///etc/passwd' }])).toBeUndefined()
		expect(pickMcpIconSrc([{ src: 'ftp://example.com/i.png' }])).toBeUndefined()
		expect(pickMcpIconSrc([{ src: 'http://example.com/i.png' }])).toBeUndefined()
	})

	// SVG can carry script, and sanitising it is not worth an icon.
	it('refuses SVG in either form', () => {
		expect(pickMcpIconSrc([{ src: 'data:image/svg+xml;base64,PHN2Zz4=' }])).toBeUndefined()
		expect(pickMcpIconSrc([{ src: 'https://example.com/logo.svg' }])).toBeUndefined()
	})

	// `mimeType` is advisory per the spec, so it must not launder the payload.
	it('judges a data URI by what it declares, not by mimeType', () => {
		expect(
			pickMcpIconSrc([{ src: 'data:image/svg+xml;base64,PHN2Zz4=', mimeType: 'image/png' }])
		).toBeUndefined()
	})

	it('refuses an oversized data URI', () => {
		expect(pickMcpIconSrc([{ src: `data:image/png;base64,${'A'.repeat(70_000)}` }])).toBeUndefined()
	})

	it('prefers the icon matching the theme, then an untagged one', () => {
		const dark = 'data:image/png;base64,ZGFyaw=='
		const icons = [{ src: png, theme: 'light' }, { src: dark, theme: 'dark' }, { src: png }]

		expect(pickMcpIconSrc(icons, true)).toBe(dark)
		expect(pickMcpIconSrc(icons, false)).toBe(png)
	})

	it('returns nothing for a missing or malformed icons field', () => {
		expect(pickMcpIconSrc(undefined)).toBeUndefined()
		expect(pickMcpIconSrc([])).toBeUndefined()
		expect(pickMcpIconSrc([{ src: 42 }, null, 'nope'])).toBeUndefined()
	})
})
