import { describe, expect, it } from 'vitest'
import { prettyPrintHtml, runDomQueryOnHtml } from './rawAppDom'

describe('prettyPrintHtml', () => {
	it('breaks a single-line subtree into indented lines', () => {
		const out = prettyPrintHtml('<div><button id="a">Go</button></div>')
		expect(out).toBe(['<div>', '  <button id="a">Go</button>', '</div>'].join('\n'))
	})

	it('nests deeper for block children', () => {
		const out = prettyPrintHtml('<ul><li>one</li><li>two</li></ul>')
		expect(out).toBe(['<ul>', '  <li>one</li>', '  <li>two</li>', '</ul>'].join('\n'))
	})

	it('does not let void elements drift the indentation', () => {
		// <img>/<input> have no closing tag; they must not increase depth.
		const out = prettyPrintHtml('<div><img src="x"><input type="text"><button>Go</button></div>')
		expect(out).toBe(
			['<div>', '  <img src="x">', '  <input type="text">', '  <button>Go</button>', '</div>'].join(
				'\n'
			)
		)
	})
})

const META = { selector: 'div', tagName: 'div', matchCount: 1 }

describe('runDomQueryOnHtml', () => {
	it('searches the rendered HTML and returns matching line numbers', async () => {
		const text = await runDomQueryOnHtml(
			'<div><button id="a">Go</button></div>',
			{
				mode: 'search',
				pattern: 'Go'
			},
			META
		)
		expect(text).toContain('Found 1 matching line(s)')
		expect(text).toContain('Go')
	})

	it('reports no matches without throwing', async () => {
		const text = await runDomQueryOnHtml(
			'<div>hello</div>',
			{
				mode: 'search',
				pattern: 'zzz-not-there'
			},
			META
		)
		expect(text).toContain('No matches')
	})

	it('reads a bounded, line-numbered window', async () => {
		const text = await runDomQueryOnHtml(
			'<div><button id="a">Go</button></div>',
			{
				mode: 'read'
			},
			META
		)
		expect(text).toContain('Showing lines 1-3 of 3')
		expect(text).toContain('<button')
	})

	it('paginates a long read with a read_dom (not read_file) continuation note', async () => {
		const items = Array.from({ length: 300 }, (_, i) => `<li>item ${i}</li>`).join('')
		const text = await runDomQueryOnHtml(`<ul>${items}</ul>`, { mode: 'read' }, META)
		expect(text).toMatch(/start_line=/)
		expect(text).toContain('read_dom')
		expect(text).not.toContain('read_file')
	})

	// Following a continuation note that omits the scope would silently re-read the
	// active app's whole body instead of the element the read was paginating through.
	it('repeats app_path and selector in the pagination note of a scoped read', async () => {
		const items = Array.from({ length: 300 }, (_, i) => `<li>item ${i}</li>`).join('')
		const text = await runDomQueryOnHtml(
			`<ul>${items}</ul>`,
			{ mode: 'read', appPath: 'f/app', selector: 'ul.list' },
			META
		)
		expect(text).toMatch(
			/Call read_dom again with app_path="f\/app", selector="ul.list", start_line=\d+/
		)
	})

	it('notes when a selector matched multiple elements', async () => {
		const text = await runDomQueryOnHtml(
			'<button>a</button>',
			{ mode: 'read' },
			{
				selector: 'button',
				tagName: 'button',
				matchCount: 3
			}
		)
		expect(text).toContain('3 elements match')
	})
})
