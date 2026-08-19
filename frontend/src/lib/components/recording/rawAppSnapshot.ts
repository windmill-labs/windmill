/**
 * DOM snapshotting for raw-app session recordings: turns a live (same-origin)
 * app document into a self-contained HTML string that renders offline in a
 * script-less iframe, plus the small helpers that describe the element a user
 * interacted with.
 */

/** Stamped on the element a step acted on, so the player can highlight it
 * without re-running a selector against a snapshot the app may have re-rendered. */
export const REC_TARGET_ATTR = 'data-wm-rec-target'

/** App authors mark sensitive nodes with this attribute: their content is left
 * out of every snapshot and their values never reach a step. */
export const NO_RECORD_ATTR = 'data-wm-no-record'

/** Hard cap on the steps a recording may hold. The player renders a row per
 * step, so the loader enforces it too on recordings it did not produce. */
export const MAX_RECORDED_STEPS = 500

/** Upper bound on any one string a step carries (its label, value, selector).
 * The recorder truncates well below this; the loader refuses more. */
export const MAX_STEP_TEXT_CHARS = 1000

/** Snapshots are whole documents; the recorder stops storing them past this, and
 * the loader refuses a recording that claims more — every frame is parsed and
 * re-serialized before it is handed to the iframe. */
export const MAX_TOTAL_FRAME_CHARS = 40 * 1024 * 1024

export const RAW_APP_INTERACTION_KINDS = [
	'click',
	'fill',
	'select',
	'toggle',
	'submit',
	'key',
	'navigate'
] as const

export type RawAppInteractionKind = (typeof RAW_APP_INTERACTION_KINDS)[number]

/** Resolve `url(...)` references of an inlined stylesheet against the sheet's
 * own URL: once its rules move into a `<style>` in the document, a relative
 * reference would otherwise resolve against the document instead of the sheet. */
export function rewriteCssUrls(css: string, sheetHref: string): string {
	const resolve = (match: string, quote: string, raw: string) => {
		const url = raw.trim()
		if (!url || /^(data:|blob:|about:|https?:|\/\/|#)/i.test(url)) return match
		try {
			return `url(${quote}${new URL(url, sheetHref).href}${quote})`
		} catch (_) {
			return match
		}
	}
	// Only real url() tokens: `content: "url(x.svg)"` is a string the replay must
	// render verbatim, so strings (and comments, which can hold anything) are
	// copied through and never scanned.
	let out = ''
	let i = 0
	while (i < css.length) {
		const ch = css[i]
		if (ch === '"' || ch === "'") {
			const end = skipString(css, i)
			out += css.slice(i, end)
			i = end
		} else if (ch === '/' && css[i + 1] === '*') {
			const end = css.indexOf('*/', i + 2)
			const stop = end === -1 ? css.length : end + 2
			out += css.slice(i, stop)
			i = stop
		} else {
			const m = /^url\(\s*(['"]?)([^'")]+)\1\s*\)/i.exec(css.slice(i))
			if (m) {
				out += resolve(m[0], m[1], m[2])
				i += m[0].length
			} else {
				out += ch
				i++
			}
		}
	}
	return out
}

/** Index just past the string starting at `start`, honouring CSS backslash
 * escapes so an escaped quote does not end it early. */
function skipString(css: string, start: number): number {
	const quote = css[start]
	let i = start + 1
	while (i < css.length) {
		if (css[i] === '\\') i += 2
		else if (css[i] === quote) return i + 1
		else i++
	}
	return css.length
}

/* The recorded document lives in another realm (the app's iframe), where
 * `instanceof Element` / `instanceof HTMLInputElement` are always false against
 * this window's constructors. Every node test here goes through nodeType/tagName
 * instead, and callers must do the same. */

/** Realm-agnostic `instanceof Element`. */
export function isElementNode(node: unknown): node is Element {
	return !!node && typeof node === 'object' && (node as Node).nodeType === 1
}

/** Realm-agnostic tag test, e.g. `isTag(el, 'INPUT')`. */
export function isTag(el: Element, tagName: string): boolean {
	return el.tagName === tagName
}

/** Never carry a typed secret into a recording that gets downloaded and shared.
 * Fixed width: the length of a masked value is itself information. */
export function maskValue(value: string): string {
	return value ? '••••••••' : ''
}

/** Index path from `root` down to `el` (element children only), so a node can be
 * located again in a structural clone of the same tree. */
function nodePath(root: Element, el: Element): number[] | undefined {
	const path: number[] = []
	let cur: Element | null = el
	while (cur && cur !== root) {
		const parent: Element | null = cur.parentElement
		if (!parent) return undefined
		path.unshift(Array.prototype.indexOf.call(parent.children, cur))
		cur = parent
	}
	return cur === root ? path : undefined
}

function resolvePath(root: Element, path: number[]): Element | undefined {
	let cur: Element | undefined = root
	for (const i of path) {
		cur = cur?.children[i] as Element | undefined
		if (!cur) return undefined
	}
	return cur
}

/** What may stay on a no-record element: enough to keep occupying the same
 * space, nothing that can carry content. An allow-list on purpose, since any
 * deny-list fails open. `class`/`id` are here by name only, their values
 * filtered by {@link styledTokens}; `style` and `checked` are deliberately not. */
const REDACTION_KEEPS_ATTRS = new Set([
	'class',
	'colspan',
	'cols',
	'disabled',
	'height',
	'hidden',
	'id',
	'multiple',
	'open',
	'readonly',
	'rows',
	'rowspan',
	'size',
	'type',
	'width'
])

/** True when the element sits under an app-declared no-record subtree. */
export function isRedacted(el: Element): boolean {
	return !!el.closest(`[${NO_RECORD_ATTR}]`)
}

/** Name a no-record element by its kind alone — its text, label and placeholder
 * are exactly what the app asked to keep out of the recording. */
export function redactedDescription(el: Element): string {
	const tag = el.tagName.toLowerCase()
	const type = (el.getAttribute('type') ?? 'text').toLowerCase()
	const role = tag === 'input' ? `input[${type}]` : tag
	return `${role} (redacted)`
}

/** A selector's identifier, escapes included and in any position: a utility
 * framework writes `md:flex` as `.md\:flex` and `2xl:block` as `.\32 xl\:block`,
 * so a matcher that stops at a backslash — or demands one before it — reads the
 * wrong token and costs a redacted placeholder the styling it is kept for. */
const IDENT_CHAR = String.raw`(?:[-\w\u00a0-\uffff]|\\[0-9a-fA-F]{1,6}[ \t\r\n\f]?|\\[^\r\n\f0-9a-fA-F])`
const CLASS_SELECTOR = new RegExp(String.raw`\.(${IDENT_CHAR}+)`, 'g')
const ID_SELECTOR = new RegExp(String.raw`#(${IDENT_CHAR}+)`, 'g')

/** CSS escapes back to the literal text an attribute holds: `\:` is `:`, and a
 * hex escape (`\3a `) is its code point. */
function unescapeCssIdent(ident: string): string {
	return ident.replace(/\\([0-9a-fA-F]{1,6})[ \t\r\n\f]?|\\([^])/g, (_, hex, ch) =>
		hex ? String.fromCodePoint(parseInt(hex, 16)) : ch
	)
}

/** Tokens the snapshot's surviving stylesheets select on. A redacted element
 * keeps `class`/`id` for its shape, but the values are author-written and can
 * name what the marker withholds; a token the CSS selects on is styling
 * vocabulary, one it never mentions buys nothing. Errs towards dropping. */
function styledTokens(clone: Element): { classes: Set<string>; ids: Set<string> } {
	const classes = new Set<string>()
	const ids = new Set<string>()
	for (const style of Array.from(clone.querySelectorAll('style'))) {
		// A marked stylesheet is scrubbed too, so its selectors are not vocabulary:
		// letting `<style data-wm-no-record>.salary-92000{}</style>` justify keeping
		// that class would launder the token the sheet was marked to withhold.
		if (isRedacted(style)) continue
		const css = style.textContent ?? ''
		for (const m of css.matchAll(CLASS_SELECTOR)) classes.add(unescapeCssIdent(m[1]))
		for (const m of css.matchAll(ID_SELECTOR)) ids.add(unescapeCssIdent(m[1]))
	}
	return { classes, ids }
}

/** Strip everything the app marked no-record: the descendants of a marked
 * element, and every attribute outside {@link REDACTION_KEEPS_ATTRS} — content
 * hides in `title`, `data-*`, `label`, `srcdoc`, a namespaced `xlink:href`, so
 * only what the replay needs for layout and control state survives. */
function redactMarkedSubtrees(doc: Document, root: Element) {
	const styled = styledTokens(root)
	// `querySelectorAll` skips the element it is called on, so a marked root
	// (`<html data-wm-no-record>`) has to be handled explicitly.
	const marked: Element[] = [
		...(root.hasAttribute(NO_RECORD_ATTR) ? [root] : []),
		...Array.from(root.querySelectorAll(`[${NO_RECORD_ATTR}]`))
	]
	for (const n of marked) {
		n.replaceChildren(doc.createTextNode('•••'))
		// The marker is kept (an app may style `[data-wm-no-record]`) but emptied: its
		// value is author-written free text that nothing downstream reads, so leaving
		// it verbatim would be the one way past the allow-list below.
		n.setAttribute(NO_RECORD_ATTR, '')
		for (const attr of Array.from(n.attributes)) {
			if (attr.name === NO_RECORD_ATTR) continue
			const name = attr.localName.toLowerCase()
			if (!REDACTION_KEEPS_ATTRS.has(name)) {
				n.removeAttributeNode(attr)
			} else if (name === 'class') {
				const kept = attr.value.split(/\s+/).filter((c) => c && styled.classes.has(c))
				if (kept.length) n.setAttribute('class', kept.join(' '))
				else n.removeAttributeNode(attr)
			} else if (name === 'id' && !styled.ids.has(attr.value)) {
				n.removeAttributeNode(attr)
			}
		}
	}
}

/** A `<select>` whose chosen option is marked cannot be serialized honestly:
 * keeping `selected` says which one was picked, dropping it says the first one
 * was. Replace its options with a single masked, selected one — the replay then
 * shows that a choice was made without disclosing it. */
/** Encoding runs synchronously on the app's own event path, several times per
 * interaction, so it is budgeted twice: no single canvas larger than this, and
 * no more than {@link MAX_SNAPSHOT_CANVAS_PIXELS} across one snapshot. A
 * dashboard of charts would otherwise stall every pointerdown, and the frame cap
 * cannot help — it is only checked once the work is already done. */
const MAX_CANVAS_PIXELS = 4_000_000
const MAX_SNAPSHOT_CANVAS_PIXELS = 8_000_000

/** A canvas holds its picture in a bitmap `outerHTML` knows nothing about, so a
 * cloned one replays blank. Paint it into the clone's own background instead of
 * swapping the element for an `<img>`, which would lose whatever the app's CSS
 * says about `canvas`. WebGL without `preserveDrawingBuffer` and cross-origin
 * tainted canvases cannot be read at all; both keep today's blank. */
function paintCanvases(doc: Document, clone: Element) {
	const live = doc.querySelectorAll('canvas')
	const copies = clone.querySelectorAll('canvas')
	if (live.length !== copies.length) return
	let budget = MAX_SNAPSHOT_CANVAS_PIXELS
	for (let i = 0; i < live.length; i++) {
		const source = live[i] as HTMLCanvasElement
		if (!source.width || !source.height) continue
		const pixels = source.width * source.height
		if (pixels > MAX_CANVAS_PIXELS || pixels > budget) continue
		budget -= pixels
		let url: string
		try {
			url = source.toDataURL('image/webp', 0.85)
		} catch (_) {
			continue // tainted by cross-origin pixels
		}
		if (!url.startsWith('data:image/')) continue
		// The replay runs without scripting, where a canvas renders its (empty)
		// fallback content: it is no longer a replaced element, so it has no intrinsic
		// size and — while it stays `display: inline` — ignores width and height
		// entirely. Carry over the box it actually rendered at, and a display that
		// accepts one.
		const rect = source.getBoundingClientRect()
		if (!rect.width || !rect.height) continue
		const display = doc.defaultView?.getComputedStyle(source).display
		const box = !display || display === 'inline' ? 'inline-block' : display
		const copy = copies[i]
		const style = copy.getAttribute('style')
		copy.setAttribute(
			'style',
			`${style ? style + ';' : ''}display:${box};box-sizing:border-box` +
				`;width:${rect.width}px;height:${rect.height}px` +
				`;background-image:url("${url}");background-size:100% 100%;background-repeat:no-repeat`
		)
	}
}

function maskSelectsWithRedactedChoice(doc: Document, clone: Element) {
	const live = doc.querySelectorAll('select')
	const copies = clone.querySelectorAll('select')
	if (live.length !== copies.length) return
	for (let i = 0; i < live.length; i++) {
		const select = live[i] as HTMLSelectElement
		if (!Array.from(select.selectedOptions).some((o) => isRedacted(o))) continue
		const masked = doc.createElement('option')
		masked.setAttribute('selected', '')
		masked.textContent = '•••'
		copies[i].replaceChildren(masked)
	}
}

/** Copy live form state (which lives in properties, not attributes, so
 * `outerHTML` would lose it) onto the clone. Passwords and anything the app
 * marked no-record are masked. */
function freezeFormState(doc: Document, clone: Element) {
	const selector = 'input, textarea, select'
	const live = doc.querySelectorAll(selector)
	const copies = clone.querySelectorAll(selector)
	if (live.length !== copies.length) return
	for (let i = 0; i < live.length; i++) {
		const el = live[i]
		const copy = copies[i]
		if (el.tagName !== copy.tagName) return
		if (isTag(el, 'INPUT')) {
			const input = el as HTMLInputElement
			const copyInput = copy as HTMLInputElement
			if (input.type === 'checkbox' || input.type === 'radio') {
				if (input.checked) copyInput.setAttribute('checked', '')
				else copyInput.removeAttribute('checked')
			} else if (input.type === 'password' || isRedacted(input)) {
				copyInput.setAttribute('value', maskValue(input.value))
			} else if (input.type !== 'file') {
				copyInput.setAttribute('value', input.value)
			}
		} else if (isTag(el, 'TEXTAREA')) {
			copy.textContent = (el as HTMLTextAreaElement).value
		} else if (isTag(el, 'SELECT')) {
			const select = el as HTMLSelectElement
			const copySelect = copy as HTMLSelectElement
			for (let j = 0; j < select.options.length; j++) {
				const option = copySelect.options[j]
				if (!option) continue
				if (select.options[j].selected) option.setAttribute('selected', '')
				else option.removeAttribute('selected')
			}
		}
	}
}

/** Rule text, with `@import` replaced by the rules it pulls in: the replay CSP
 * refuses the fetch, so an unexpanded import is simply lost styling. An import
 * that carries a cascade layer replays as unlayered rules at the import's
 * position — closer to the live rendering than dropping it, but not identical. */
function expandRules(rules: CSSRuleList): string {
	return Array.from(rules)
		.map((rule) => {
			const imported = (rule as CSSImportRule).styleSheet
			if (!imported) return rule.cssText
			try {
				const inner = expandRules(imported.cssRules)
				const media = imported.media?.mediaText
				return media ? `@media ${media} {\n${inner}\n}` : inner
			} catch (_) {
				return rule.cssText
			}
		})
		.join('\n')
}

/** Re-read every time rather than cached against a sampled probe: an app that
 * restyles at runtime (a theme toggle rewriting a rule in place) leaves any
 * sample of the sheet unchanged, and the snapshot would then replay styling the
 * user never saw. A probe cheap enough to be worth caching cannot be exact,
 * because the cost being avoided *is* reading the rules. Measured at ~18ms for
 * 11k rules across 177 sheets, which is small next to the rest of a snapshot. */
function sheetCss(sheet: CSSStyleSheet, rules: CSSRuleList): string {
	let css = expandRules(rules)
	if (sheet.href) css = rewriteCssUrls(css, sheet.href)
	// `cssRules` drops the sheet-level media condition the `<link media>` carried,
	// so an unwrapped inline copy would apply print-only CSS to every replay.
	const media = sheet.media?.mediaText
	if (media) css = `@media ${media} {\n${css}\n}`
	return css
}

/** Inline what the browser has actually parsed: rules of linked stylesheets (so
 * the snapshot renders without the API being reachable) and of CSS-in-JS sheets
 * built with `insertRule` (whose `<style>` node clones out empty). Sheets we
 * can't read (cross-origin) keep their `<link>`, and ones with no owner node
 * (`adoptedStyleSheets`) are out of reach entirely — as is anything inside a
 * shadow root, which `outerHTML` does not serialize. */
function inlineStyleSheets(doc: Document, root: Element, clone: Element) {
	for (const sheet of Array.from(doc.styleSheets)) {
		const owner = sheet.ownerNode
		if (!isElementNode(owner)) continue
		if (sheet.disabled) {
			// `disabled` is a property, not an attribute: cloned through, the sheet
			// would come back to life on replay. Neutralize it in place — removing the
			// node would shift the sibling indices that every later path resolution in
			// this function, and the target stamp after it, resolve against.
			const path = nodePath(root, owner)
			const target = path ? resolvePath(clone, path) : undefined
			if (target) {
				target.setAttribute('media', 'not all')
				if (target.tagName === 'STYLE') target.textContent = ''
			}
			continue
		}
		// Inlining replaces the node, which would leave the marker behind and carry
		// the sheet's text into the snapshot. Leave marked sheets for redaction.
		if (isRedacted(owner)) continue
		let rules: CSSRuleList
		try {
			const cssRules = (sheet as CSSStyleSheet).cssRules
			if (!cssRules) continue
			rules = cssRules
		} catch (_) {
			continue
		}
		const path = nodePath(root, owner)
		if (!path) continue
		const target = resolvePath(clone, path)
		if (!target) continue
		const css = sheetCss(sheet as CSSStyleSheet, rules)
		if (owner.tagName === 'LINK') {
			const style = doc.createElement('style')
			style.textContent = css
			target.replaceWith(style)
		} else if (owner.tagName === 'STYLE') {
			// The parsed rules are the truth: `insertRule`/`deleteRule` and edits to a
			// rule's style never touch the element's source text, so copying the text
			// through would replay the sheet as it was authored, not as it renders.
			target.textContent = css
		}
	}
}

export type SnapshotOptions = {
	/** Element to stamp with {@link REC_TARGET_ATTR} (the step's interaction target). */
	target?: Element | null
	/** Base URL for the snapshot's relative resources (the recording origin). */
	baseHref?: string
}

/** Serialize a live document into standalone HTML: current form state frozen in,
 * stylesheets inlined, scripts and inline handlers dropped (the player renders
 * snapshots with scripting disabled). */
export function serializeDocument(doc: Document, opts: SnapshotOptions = {}): string {
	const root = doc.documentElement
	const clone = root.cloneNode(true) as Element
	// These read the live document and write to the matching node in the clone,
	// pairing by position — so they must all run before anything is removed from
	// it. Freezing, inlining, painting and redacting only mutate nodes in place.
	freezeFormState(doc, clone)
	inlineStyleSheets(doc, root, clone)
	paintCanvases(doc, clone)
	maskSelectsWithRedactedChoice(doc, clone)
	redactMarkedSubtrees(doc, clone)
	// Stamp the target BEFORE anything is removed from the clone: the live tree is
	// what `nodePath` indexes against, so a single removed node (a data `<script>`
	// preceding the target, say) would shift every later sibling and stamp the
	// wrong element.
	if (opts.target) {
		const path = nodePath(root, opts.target)
		const target = path ? resolvePath(clone, path) : undefined
		target?.setAttribute(REC_TARGET_ATTR, '')
	}
	// Templates render nothing, and their content fragment is invisible to
	// `querySelectorAll` while `outerHTML` still serializes it — so the passes
	// below would miss scripts and handlers hiding inside one. `<noscript>` is the
	// same trap from the other side: with scripting on its markup is one text node,
	// so redaction cannot see into it, yet it would render on a script-less replay.
	clone.querySelectorAll('template, noscript').forEach((n) => n.remove())
	clone.querySelectorAll('script').forEach((n) => n.remove())
	clone.querySelectorAll('meta[http-equiv="refresh" i]').forEach((n) => n.remove())
	clone.querySelectorAll('*').forEach((el) => {
		for (const attr of Array.from(el.attributes)) {
			if (attr.name.toLowerCase().startsWith('on')) el.removeAttribute(attr.name)
		}
	})
	// A snapshot clones out scrolled back to the top, which can leave the
	// interaction target off-screen on replay. Shifting the root reproduces the
	// scrolled view (and leaves `position: fixed` chrome where it belongs). Scroll
	// inside nested overflow containers has no static-CSS equivalent and is lost.
	const view = doc.defaultView
	const scrollY = Math.round(view?.scrollY ?? doc.documentElement.scrollTop ?? 0)
	const scrollX = Math.round(view?.scrollX ?? doc.documentElement.scrollLeft ?? 0)
	if (scrollY > 0 || scrollX > 0) {
		const scrolled = doc.createElement('style')
		scrolled.textContent = `html { margin-top: -${scrollY}px !important; margin-left: -${scrollX}px !important; }`
		clone.querySelector('head')?.appendChild(scrolled)
	}
	const head = clone.querySelector('head')
	if (opts.baseHref && head && !head.querySelector('base')) {
		const base = doc.createElement('base')
		base.setAttribute('href', opts.baseHref)
		head.prepend(base)
	}
	return `<!DOCTYPE html>${clone.outerHTML}`
}

const NAVIGATION_ATTRS = new Set(['href', 'action', 'formaction', 'ping', 'target', 'download'])

/** Locked-down policy for a replayed snapshot. The player's empty sandbox stops
 * scripting, but not subresource loads: without this, markup inside a recording
 * fetched from an arbitrary `?src=` URL could still beacon the viewer or issue
 * same-site GETs against their Windmill session. The cost is that images and
 * fonts the recorder could not inline (remote URLs) do not render on replay.
 * Injected at the very top of <head> so it applies before anything is fetched. */
const REPLAY_CSP = `default-src 'none'; style-src 'unsafe-inline'; img-src data:; font-src data:`

/** A replay is a picture of a past session, not a working app, so clicks are
 * turned off at the root. `pointer-events` inherits rather than cascades, so an
 * element that sets its own value (Tailwind's `pointer-events-auto`, overlay
 * CSS) still takes clicks — this is a broad default, NOT a guarantee, and the
 * attribute stripping below remains the actual defense. It also costs
 * mouse text-selection inside the replayed frame. */
const INERT_CSS = `html { pointer-events: none !important; }`

const HIGHLIGHT_CSS = `[${REC_TARGET_ATTR}] {
	outline: 3px solid #ef4444 !important;
	outline-offset: 2px !important;
	box-shadow: 0 0 0 6px rgba(239, 68, 68, 0.25) !important;
}`

/** Prepare a recorded frame for replay: policy first in `<head>`, target
 * highlight, nothing executable. Parsed rather than string-spliced because a
 * `?src=` recording can defeat a `<head>` regex (`<body><img src=/probe><header>`)
 * and land the policy after the request it must prevent. Not in the player: a
 * literal `<style>` there is parsed as the component's own style block. */
export function withHighlightStyles(frame: string): string {
	let doc: Document
	try {
		doc = new DOMParser().parseFromString(frame, 'text/html')
	} catch (_) {
		return ''
	}
	const head = doc.head ?? doc.documentElement.insertBefore(doc.createElement('head'), doc.body)
	const csp = doc.createElement('meta')
	csp.setAttribute('http-equiv', 'Content-Security-Policy')
	csp.setAttribute('content', REPLAY_CSP)
	head.prepend(csp)
	// The recorder strips these at capture time; a hand-made or hostile recording
	// has not been through it.
	doc.querySelectorAll('script, meta[http-equiv="refresh" i]').forEach((n) => n.remove())
	doc.querySelectorAll('*').forEach((el) => {
		for (const attr of Array.from(el.attributes)) {
			if (attr.name.toLowerCase().startsWith('on')) el.removeAttribute(attr.name)
		}
	})
	// A replay is a static picture, so nothing in it may navigate: the sandbox
	// stops a *top-level* navigation and the CSP governs fetches, but neither stops
	// a link from navigating the snapshot frame itself — which is a request.
	// Matched on the attribute's local name, so an SVG `xlink:href` goes too.
	doc.querySelectorAll('a, area, form, button, input').forEach((el) => {
		for (const attr of Array.from(el.attributes)) {
			if (NAVIGATION_ATTRS.has(attr.localName.toLowerCase())) el.removeAttributeNode(attr)
		}
	})
	// SVG animation is declarative — it runs without scripts — and `<set
	// attributeName="href">` would put back the link just stripped.
	doc.querySelectorAll('set, animate, animateTransform, animateMotion').forEach((n) => n.remove())
	// `querySelectorAll` cannot see into a template's content fragment, so markup
	// hidden there escapes every pass above and comes alive as a shadow root when
	// the frame is parsed. A snapshot has no use for templates: drop them.
	doc.querySelectorAll('template').forEach((n) => n.remove())
	// Resource hints exist only to fetch; the CSP already refuses them, so this is
	// about not asking rather than not getting.
	doc
		.querySelectorAll(
			'link[rel~="preload" i], link[rel~="prefetch" i], link[rel~="preconnect" i], link[rel~="dns-prefetch" i]'
		)
		.forEach((n) => n.remove())
	const style = doc.createElement('style')
	style.textContent = `${INERT_CSS}\n${HIGHLIGHT_CSS}`
	head.appendChild(style)
	return `<!DOCTYPE html>${doc.documentElement.outerHTML}`
}

/** Text of an element with any no-record subtree left out: the element itself
 * may be recordable while something inside it is not. */
export function textWithoutRedacted(el: Element | null | undefined): string {
	if (!el || isRedacted(el)) return ''
	let source: Element = el
	if (el.querySelector(`[${NO_RECORD_ATTR}]`)) {
		source = el.cloneNode(true) as Element
		source.querySelectorAll(`[${NO_RECORD_ATTR}]`).forEach((n) => n.remove())
	}
	return (source.textContent ?? '').replace(/\s+/g, ' ').trim()
}

function textOf(el: Element | null | undefined, max = 40): string {
	const text = textWithoutRedacted(el)
	return text.length > max ? `${text.slice(0, max)}…` : text
}

/** Short human name of an element, preferring what a user would call it
 * (its label / accessible name) over its markup. */
export function describeElement(el: Element): string {
	const tag = el.tagName.toLowerCase()
	const type = (el.getAttribute('type') ?? 'text').toLowerCase()
	const role = tag === 'input' ? `input[${type}]` : tag
	// An element can be recordable while its associated label is not (the label is
	// where a form usually puts the sensitive wording).
	const label = (el as HTMLInputElement).labels?.[0]
	const name =
		el.getAttribute('aria-label') ||
		(label && !isRedacted(label) ? textOf(label) : '') ||
		// A button-shaped <input> has no text content: its `value` is its caption.
		(tag === 'input' && ['button', 'submit', 'reset'].includes(type)
			? el.getAttribute('value')
			: '') ||
		el.getAttribute('placeholder') ||
		el.getAttribute('title') ||
		textOf(el) ||
		el.getAttribute('name') ||
		el.getAttribute('id') ||
		''
	return name ? `${role} "${name}"` : role
}

/** Best-effort CSS selector for the element, recorded for reference (the player
 * highlights via {@link REC_TARGET_ATTR}, not this). */
export function cssSelectorFor(el: Element): string {
	const parts: string[] = []
	let cur: Element | null = el
	let depth = 0
	while (cur && depth < 5) {
		const tag = cur.tagName.toLowerCase()
		if (cur.id) {
			parts.unshift(`#${cur.id}`)
			break
		}
		const cls =
			typeof cur.className === 'string'
				? cur.className.trim().split(/\s+/).filter(Boolean)[0]
				: undefined
		const parent: Element | null = cur.parentElement
		let part = cls ? `${tag}.${cls}` : tag
		if (parent) {
			const sameTag = Array.from(parent.children).filter((c) => c.tagName === cur!.tagName)
			if (sameTag.length > 1) part += `:nth-of-type(${sameTag.indexOf(cur) + 1})`
		}
		parts.unshift(part)
		cur = parent
		depth++
	}
	return parts.join(' > ')
}

/** One-line description of a step, shown in the player's step list. */
export function stepLabel(kind: RawAppInteractionKind, target: string, value?: string): string {
	switch (kind) {
		case 'click':
			return `Clicked ${target}`
		case 'fill':
			return `Filled ${target} with "${value ?? ''}"`
		case 'select':
			return `Selected "${value ?? ''}" in ${target}`
		case 'toggle':
			// A redacted control reports no state, and "Unchecked" would then be a
			// claim rather than an omission: say only that it was toggled.
			if (!value) return `Toggled ${target}`
			return `${value === 'checked' ? 'Checked' : 'Unchecked'} ${target}`
		case 'submit':
			return `Submitted ${target}`
		case 'key':
			return `Pressed ${value ?? 'key'} in ${target}`
		case 'navigate':
			return value ? `Navigated to ${value}` : 'Reloaded the app'
	}
}
