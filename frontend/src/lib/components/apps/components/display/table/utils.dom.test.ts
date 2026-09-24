import { describe, expect, it } from 'vitest'
import type { ICellRendererParams } from 'ag-grid-community'
import { defaultCellRenderer } from './utils'

function render(value: unknown) {
	const renderer = defaultCellRenderer('link')!
	return renderer({ value } as ICellRendererParams) as HTMLElement
}

// AG Grid writes a renderer's string output to innerHTML, and link cells hold run results.
describe('link cell renderer', () => {
	it('renders a link without parsing its label or href as markup', () => {
		const el = render({ href: 'https://x.test/"><img src=x onerror=alert(1)>', label: '<b>hi</b>' })
		expect(el.tagName).toBe('A')
		expect(el.textContent).toBe('<b>hi</b>')
		expect(el.getAttribute('href')).toBe('https://x.test/"><img src=x onerror=alert(1)>')
		expect(el.children).toHaveLength(0)
	})

	it.each([
		{ href: 'javascript:alert(1)', label: 'click' },
		'javascript:alert(1)',
		{ href: 'data:text/html,<script>alert(1)</script>', label: 'click' }
	])('renders a non-navigating href %j as plain text', (value) => {
		const el = render(value)
		expect(el.tagName).toBe('SPAN')
		expect(el.hasAttribute('href')).toBe(false)
	})
})
