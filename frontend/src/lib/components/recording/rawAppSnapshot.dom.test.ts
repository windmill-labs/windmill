/**
 * The two contracts of this module that are worth pinning: a snapshot never
 * carries what the app marked no-record, and a replayed frame can neither run
 * nor fetch nor navigate. Both are non-obvious against the DOM (a template's
 * content is serialized but not queryable, attributes match on local name, the
 * policy has to be structurally first) and both are where a later simplification
 * would silently reopen something.
 */
import { describe, expect, it } from 'vitest'
import { serializeDocument, withHighlightStyles } from './rawAppSnapshot'

function docFrom(body: string): Document {
	return new DOMParser().parseFromString(
		`<html><head></head><body>${body}</body></html>`,
		'text/html'
	)
}

describe('serializeDocument redaction', () => {
	it('drops the content, attributes and template fragment of a no-record element', () => {
		const doc = docFrom(
			`<div data-wm-no-record="why-hidden-92000" title="salary 92000" data-token="sk-secret" aria-label="confidential">visible secret</div>` +
				`<select><option data-wm-no-record label="codename falcon">falcon</option></select>` +
				`<template data-wm-no-record><input value="template secret"></template>`
		)
		const svgLink = doc.createElementNS('http://www.w3.org/2000/svg', 'image')
		svgLink.setAttribute('data-wm-no-record', '')
		svgLink.setAttributeNS(
			'http://www.w3.org/1999/xlink',
			'xlink:href',
			'https://host/signed-secret'
		)
		doc.body.appendChild(svgLink)

		const html = serializeDocument(doc)
		for (const secret of [
			'visible secret',
			'salary 92000',
			'sk-secret',
			'confidential',
			'codename falcon',
			'falcon',
			'template secret',
			'signed-secret',
			// The marker itself is author-written free text; it survives so the element
			// can still be styled, but carries nothing of its own.
			'why-hidden-92000'
		]) {
			expect(html).not.toContain(secret)
		}
		expect(html).toContain('data-wm-no-record=""')
	})

	it('redacts a marked document root down to its constrained attributes', () => {
		// Marking the root marks every stylesheet with it, so no CSS survives to
		// justify keeping a class: only attributes whose values cannot carry content
		// are left.
		const doc = docFrom(
			`<style>.theme-dark { color: white }</style><p>everything here is private</p>`
		)
		doc.documentElement.setAttribute('data-wm-no-record', '')
		doc.documentElement.setAttribute('class', 'theme-dark')
		doc.documentElement.setAttribute('hidden', '')
		doc.documentElement.setAttribute('cite', 'https://host/private-source')

		const html = serializeDocument(doc)
		expect(html).not.toContain('everything here is private')
		expect(html).not.toContain('private-source')
		expect(html).not.toContain('theme-dark')
		expect(html).toContain('hidden')
	})

	it('keeps no attribute that could carry content, listed or not', () => {
		const doc = docFrom(
			`<style>.frame { border: 1px solid }</style>` +
				`<iframe data-wm-no-record srcdoc="<p>embedded secret</p>" data-anything="future secret"` +
				` cite="/cited" style="--code: salary-92000" class="frame"></iframe>`
		)
		const html = serializeDocument(doc)
		expect(html).not.toContain('embedded secret')
		expect(html).not.toContain('future secret')
		expect(html).not.toContain('/cited')
		expect(html).not.toContain('salary-92000')
		// The layout-bearing class survives: the snapshot's own CSS selects on it.
		expect(html).toContain('class="frame"')
	})

	it('does not let a marked stylesheet justify keeping the class it selects', () => {
		// The marked sheet is scrubbed, so its selectors are not part of the
		// snapshot's vocabulary: honouring them would launder the very token the
		// author marked the sheet to withhold.
		const doc = docFrom(
			`<style data-wm-no-record>.salary-92000 { color: red }</style>` +
				`<div data-wm-no-record class="salary-92000">x</div>`
		)
		const html = serializeDocument(doc)
		expect(html).not.toContain('salary-92000')
	})

	it('keeps only the class and id tokens the snapshot styles', () => {
		// `class` and `id` stay on a redacted element so its box keeps its shape,
		// but the values are the app's to choose and can name what the marker hides.
		const doc = docFrom(
			`<style>.card { padding: 4px }</style>` +
				`<div data-wm-no-record class="card customer-acme-secret" id="salary-92000">x</div>`
		)
		const html = serializeDocument(doc)
		expect(html).toContain('class="card"')
		expect(html).not.toContain('customer-acme-secret')
		expect(html).not.toContain('salary-92000')
	})

	it('withholds even the state of a redacted control', () => {
		// Whether a marked box is ticked is exactly what the marker exists to hide;
		// the step's value is masked to match, so label and snapshot agree.
		const doc = docFrom(`<input type="checkbox" data-wm-no-record aria-label="acquisition target">`)
		const box = doc.querySelector('input') as HTMLInputElement
		box.checked = true

		const html = serializeDocument(doc)
		expect(html).not.toContain('checked')
		expect(html).not.toContain('acquisition target')
	})

	it('does not launder a marked stylesheet into the snapshot by inlining it', () => {
		// The inliner exists for sheets whose rules live only in the CSSOM (an empty
		// `<style>` filled with `insertRule`, or a `<link>`); it must skip a marked
		// owner, or the replacement node would carry the CSS past redaction.
		// The live document, not a parsed one: only a document with a browsing
		// context exposes `style.sheet`, which is what the inliner reads.
		const style = document.createElement('style')
		style.setAttribute('data-wm-no-record', '')
		document.head.appendChild(style)
		try {
			style.sheet?.insertRule(`.a { content: "sentinel-92000"; }`, 0)
			expect(style.sheet?.cssRules.length).toBe(1)
			expect(style.textContent).toBe('')

			expect(serializeDocument(document)).not.toContain('sentinel-92000')
		} finally {
			style.remove()
		}
	})

	it('snapshots a stylesheet as it renders, not as it was authored', () => {
		// `insertRule` and friends never touch the element's text, so a <style> that
		// shipped with CSS and was then mutated at runtime would replay stale.
		const style = document.createElement('style')
		style.textContent = `.authored { color: red; }`
		document.head.appendChild(style)
		try {
			style.sheet?.insertRule(`.added-at-runtime { color: blue; }`, 0)
			const html = serializeDocument(document)
			expect(html).toContain('added-at-runtime')
			expect(html).toContain('authored')
		} finally {
			style.remove()
		}
	})

	it('keeps a utility class whose selector starts with an escape', () => {
		// `2xl:block` compiles to `.\\32 xl\\:block` and `!flex` to `.\\!flex`; demanding an
		// ASCII first character drops both from the placeholder.
		const doc = docFrom(
			`<style>.\\32 xl\\:block { display: block } .\\!flex { display: flex }</style>` +
				`<div data-wm-no-record class="2xl:block !flex">x</div>`
		)
		const html = serializeDocument(doc)
		expect(html).toContain('class="2xl:block !flex"')
	})

	it('keeps a utility class whose selector is escaped', () => {
		// A framework writes `md:flex` as `.md\\:flex`; reading the selector up to the
		// backslash would drop the real token and leave the placeholder unstyled.
		const doc = docFrom(
			`<style>.md\\:flex { display: flex } .w-1\\/2 { width: 50% }</style>` +
				`<div data-wm-no-record class="md:flex w-1/2 not-styled-92000">x</div>`
		)
		const html = serializeDocument(doc)
		expect(html).toContain('class="md:flex w-1/2"')
		expect(html).not.toContain('not-styled-92000')
	})

	it('paints a canvas into the snapshot, but never a redacted one', () => {
		// A canvas keeps its picture in a bitmap `outerHTML` cannot see, so without
		// this the chart replays blank. A marked one must stay blank: the painted
		// background rides on `style`, which redaction strips.
		const doc = docFrom(
			`<canvas id="chart"></canvas><div data-wm-no-record><canvas id="secret"></canvas></div>`
		)
		for (const c of Array.from(doc.querySelectorAll('canvas'))) {
			Object.defineProperty(c, 'width', { value: 200 })
			Object.defineProperty(c, 'height', { value: 100 })
			Object.defineProperty(c, 'toDataURL', {
				value: () => `data:image/webp;base64,PIXELS-${c.id}`
			})
			Object.defineProperty(c, 'getBoundingClientRect', {
				value: () => ({ width: 200, height: 100 })
			})
		}

		const html = serializeDocument(doc)
		expect(html).toContain('PIXELS-chart')
		expect(html).not.toContain('PIXELS-secret')
	})

	it('stops encoding once a snapshot has spent its canvas budget', () => {
		// Encoding is synchronous and on the app's event path, so a wall of charts
		// must not each cost an encode — the per-canvas cap alone would allow it.
		const doc = docFrom(
			Array.from({ length: 6 }, (_, i) => `<canvas id="c${i}"></canvas>`).join('')
		)
		const encoded: string[] = []
		for (const c of Array.from(doc.querySelectorAll('canvas'))) {
			Object.defineProperty(c, 'width', { value: 2000 })
			Object.defineProperty(c, 'height', { value: 1500 }) // 3M pixels each
			Object.defineProperty(c, 'getBoundingClientRect', {
				value: () => ({ width: 200, height: 150 })
			})
			Object.defineProperty(c, 'toDataURL', {
				value: () => {
					encoded.push(c.id)
					return `data:image/webp;base64,PIXELS-${c.id}`
				}
			})
		}

		serializeDocument(doc)
		expect(encoded).toEqual(['c0', 'c1'])
	})

	it('keeps a disabled sheet inert without shifting what follows it', () => {
		// Neutralizing a disabled sheet must not remove its node: every later path
		// resolution — other sheets, and the target stamp — is by sibling index.
		const off = document.createElement('style')
		off.textContent = `.disabled-rule { color: red; }`
		const on = document.createElement('style')
		document.head.append(off, on)
		try {
			off.sheet!.disabled = true
			on.sheet?.insertRule(`.live-rule { color: green; }`, 0)

			const html = serializeDocument(document)
			expect(html).toContain('live-rule')
			expect(html).not.toContain('disabled-rule')
			expect(html).toContain('media="not all"')
		} finally {
			off.remove()
			on.remove()
		}
	})

	it('drops <noscript> rather than trusting the redaction pass to see into it', () => {
		// With scripting on, a <noscript>'s markup is one text node — invisible to
		// the redaction pass, but `outerHTML` writes it back out. (A parser with
		// scripting off, like this one, exposes it as elements instead, so the
		// assertion that matters is that the element does not survive at all.)
		const doc = docFrom(`<noscript><div data-wm-no-record>noscript secret</div></noscript>`)
		const html = serializeDocument(doc)
		expect(html).not.toContain('<noscript')
		expect(html).not.toContain('noscript secret')
	})

	it('shows that a redacted option was chosen without saying which', () => {
		// The first select sits inside a marked container, whose subtree redaction
		// drops it from the clone entirely: the masking must already have run by
		// then, or pairing by index would silently skip every select in the document.
		const doc = docFrom(
			`<div data-wm-no-record><select><option>hidden</option></select></div>` +
				`<select><option>Public</option><option data-wm-no-record>Confidential case 92000</option></select>`
		)
		const select = doc.querySelectorAll('select')[1] as HTMLSelectElement
		select.selectedIndex = 1

		const html = serializeDocument(doc)
		expect(html).not.toContain('Confidential case 92000')
		// Neither option is asserted as the choice; one masked entry stands in.
		expect(html).not.toContain('Public')
		expect(html).toContain('•••')
	})

	it('masks a password without leaking its length, and keeps other values', () => {
		const doc = docFrom(`<input type="password"><input type="text">`)
		const [password, text] = Array.from(doc.querySelectorAll('input')) as HTMLInputElement[]
		password.value = 'hunter2-hunter2-hunter2'
		text.value = 'ordinary'

		const html = serializeDocument(doc)
		expect(html).not.toContain('hunter2')
		expect(html).toContain('••••••••')
		expect(html).toContain('ordinary')
	})

	it('stamps the interaction target even when a removed node precedes it', () => {
		// The stamp is resolved by child index, so it has to be applied before the
		// script/template removals shift later siblings.
		const doc = docFrom(`<script type="application/json">{}</script><button id="go">Go</button>`)
		const target = doc.getElementById('go')!

		const html = serializeDocument(doc, { target })
		expect(html).toMatch(/<button id="go" data-wm-rec-target=""/)
		expect(html).not.toContain('<script')
	})
})

describe('withHighlightStyles', () => {
	it('puts the policy ahead of markup a <head> match would step over', () => {
		const out = withHighlightStyles(`<body><img src="/probe"><header>hi</header></body>`)
		expect(out.indexOf('Content-Security-Policy')).toBeLessThan(out.indexOf('/probe'))
	})

	it('leaves nothing that can execute, fetch or navigate', () => {
		const out = withHighlightStyles(
			`<html><head><meta http-equiv="refresh" content="0;url=/x"></head><body>` +
				`<script>fetch('/x')</script><div onclick="fetch('/y')">hi</div>` +
				`<a href="/nav">link</a><form action="/post"><button formaction="/post2">go</button></form>` +
				`<svg><a xlink:href="/svg-nav"><set attributeName="href" to="/smil-nav"/></a></svg>` +
				`<div><template shadowrootmode="open"><a href="/shadow-nav">shadow</a></template></div>` +
				`<link rel="preload" href="/pre" as="script"></body></html>`
		)
		for (const gone of [
			'<script',
			'onclick',
			'refresh',
			'href=',
			'action=',
			'<set',
			'<template',
			'preload'
		]) {
			expect(out).not.toContain(gone)
		}
	})
})
