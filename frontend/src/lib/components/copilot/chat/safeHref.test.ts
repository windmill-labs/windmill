import { describe, expect, it } from 'vitest'
import { safeHref } from './safeHref'

const BASE = 'https://app.example.com/sessions?workspace=demo'

describe('safeHref', () => {
	it.each([
		'https://windmill.dev/docs',
		'http://localhost:3000/',
		'mailto:someone@example.com',
		'/runs/abc',
		'#anchor',
		'docs/page'
	])('keeps %s', (href) => {
		expect(safeHref(href, BASE)).toBe(href)
	})

	it.each([
		'javascript:alert(1)',
		'JavaScript:alert(1)',
		' javascript:alert(1)',
		'data:text/html,<script>alert(1)</script>',
		'vbscript:msgbox',
		'file:///etc/passwd'
	])('drops %s', (href) => {
		expect(safeHref(href, BASE)).toBeUndefined()
	})

	it('drops a missing or empty href', () => {
		expect(safeHref(undefined, BASE)).toBeUndefined()
		expect(safeHref('', BASE)).toBeUndefined()
	})
})
