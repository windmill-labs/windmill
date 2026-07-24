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

export type RawAppInteractionKind =
	| 'click'
	| 'fill'
	| 'select'
	| 'toggle'
	| 'submit'
	| 'key'
	| 'navigate'

/** Resolve `url(...)` references of an inlined stylesheet against the sheet's
 * own URL: once its rules move into a `<style>` in the document, a relative
 * reference would otherwise resolve against the document instead of the sheet. */
export function rewriteCssUrls(css: string, sheetHref: string): string {
	return css.replace(/url\(\s*(['"]?)([^'")]+)\1\s*\)/g, (match, quote: string, raw: string) => {
		const url = raw.trim()
		if (!url || /^(data:|blob:|about:|https?:|\/\/|#)/i.test(url)) return match
		try {
			return `url(${quote}${new URL(url, sheetHref).href}${quote})`
		} catch (_) {
			return match
		}
	})
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

/** Never carry a typed secret into a recording that gets downloaded and shared. */
export function maskValue(value: string): string {
	return '•'.repeat(Math.min(value.length, 12))
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

/** Attributes that carry content rather than presentation, and so must not
 * survive on a no-record element. */
const REDACTED_ATTRS = new Set([
	'alt',
	'action',
	'download',
	'formaction',
	'href',
	'placeholder',
	'poster',
	'src',
	'srcset',
	'style',
	'title',
	'value'
])

/** True when the element sits under an app-declared no-record subtree. */
export function isRedacted(el: Element): boolean {
	return !!el.closest(`[${NO_RECORD_ATTR}]`)
}

/** Name a no-record element by its kind alone — its text, label and placeholder
 * are exactly what the app asked to keep out of the recording. */
export function redactedDescription(el: Element): string {
	const tag = el.tagName.toLowerCase()
	const role = tag === 'input' ? `input[${(el.getAttribute('type') ?? 'text').toLowerCase()}]` : tag
	return `${role} (redacted)`
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

/** Re-stringifying every rule of a framework stylesheet costs more than the rest
 * of a snapshot combined, and snapshots are taken several times per interaction
 * on the app's own event path. Only linked sheets are cached: their text is
 * fixed once fetched, whereas a CSS-in-JS `<style>` is rewritten in place (a
 * theme toggle can change a rule without changing the rule count). */
const sheetCssCache = new WeakMap<CSSStyleSheet, { probe: string; css: string }>()

/** Cheap stand-in for "have these rules changed": the count plus the text of up
 * to 16 rules spread across the sheet (every rule, for a small one). Reading a
 * rule stringifies it, so probing all of a framework sheet would cost as much as
 * the re-serialization this cache exists to avoid — a mutation to an unsampled
 * rule of a *linked* sheet can therefore be missed, which is why only linked
 * sheets (fetched once, rarely rewritten) are cached at all. */
function rulesProbe(rules: CSSRuleList): string {
	const n = rules.length
	const samples = Math.min(16, n)
	const parts = [String(n)]
	for (let i = 0; i < samples; i++) {
		const index = samples === 1 ? 0 : Math.round((i * (n - 1)) / (samples - 1))
		const text = rules[index]?.cssText ?? ''
		parts.push(String(text.length), text.slice(0, 64))
	}
	return parts.join('|')
}

function sheetCss(sheet: CSSStyleSheet, rules: CSSRuleList): string {
	const cacheable = !!sheet.href
	const probe = cacheable ? rulesProbe(rules) : ''
	const cached = cacheable ? sheetCssCache.get(sheet) : undefined
	if (cached && cached.probe === probe) return cached.css
	let css = Array.from(rules)
		.map((r) => r.cssText)
		.join('\n')
	if (sheet.href) css = rewriteCssUrls(css, sheet.href)
	// `cssRules` drops the sheet-level media condition the `<link media>` carried,
	// so an unwrapped inline copy would apply print-only CSS to every replay.
	const media = sheet.media?.mediaText
	if (media) css = `@media ${media} {\n${css}\n}`
	if (cacheable) sheetCssCache.set(sheet, { probe, css })
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
		if (!isElementNode(owner) || sheet.disabled) continue
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
		} else if (owner.tagName === 'STYLE' && (target.textContent ?? '').trim() === '') {
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
	freezeFormState(doc, clone)
	inlineStyleSheets(doc, root, clone)
	// Stamp the target BEFORE anything is removed from the clone: the live tree is
	// what `nodePath` indexes against, so a single removed node (a data `<script>`
	// preceding the target, say) would shift every later sibling and stamp the
	// wrong element. Freezing and inlining above only mutate nodes in place.
	if (opts.target) {
		const path = nodePath(root, opts.target)
		const target = path ? resolvePath(clone, path) : undefined
		target?.setAttribute(REC_TARGET_ATTR, '')
	}
	clone.querySelectorAll('script').forEach((n) => n.remove())
	// The app declared these subtrees off-limits: keep the node (so the layout
	// still replays) but never carry their content into the recording — neither
	// their descendants nor the attributes that are content themselves (a signed
	// `src`, a `title`, a `data-*` payload).
	clone.querySelectorAll(`[${NO_RECORD_ATTR}]`).forEach((n) => {
		n.replaceChildren(doc.createTextNode('•••'))
		for (const attr of Array.from(n.attributes)) {
			const name = attr.name.toLowerCase()
			if (name === NO_RECORD_ATTR || name === REC_TARGET_ATTR) continue
			if (REDACTED_ATTRS.has(name) || name.startsWith('data-') || name.startsWith('aria-')) {
				n.removeAttribute(attr.name)
			}
		}
	})
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
	const scrollY = Math.round(doc.defaultView?.scrollY ?? doc.documentElement.scrollTop ?? 0)
	if (scrollY > 0) {
		const scrolled = doc.createElement('style')
		scrolled.textContent = `html { margin-top: -${scrollY}px !important; }`
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

/** Locked-down policy for a replayed snapshot. The player's empty sandbox stops
 * scripting, but not subresource loads: without this, markup inside a recording
 * fetched from an arbitrary `?src=` URL could still beacon the viewer or issue
 * same-site GETs against their Windmill session. The cost is that images and
 * fonts the recorder could not inline (remote URLs) do not render on replay.
 * Injected at the very top of <head> so it applies before anything is fetched. */
const REPLAY_CSP = `default-src 'none'; style-src 'unsafe-inline'; img-src data:; font-src data:`

const HIGHLIGHT_CSS = `[${REC_TARGET_ATTR}] {
	outline: 3px solid #ef4444 !important;
	outline-offset: 2px !important;
	box-shadow: 0 0 0 6px rgba(239, 68, 68, 0.25) !important;
}`

/** Prepare a recorded frame for replay: the policy first in <head>, then the
 * target highlight, and anything executable dropped.
 *
 * Goes through a real parser rather than string surgery — a recording can come
 * from an arbitrary `?src=` URL, and markup like `<body><img src=/probe><header>`
 * defeats a `<head>` regex, putting the policy after the request it must prevent.
 * DOMParser does not fetch subresources or run scripts, so parsing is inert; the
 * document only becomes live when the player hands it to a sandboxed iframe.
 *
 * Lives here rather than in the player because a literal `<style>` tag inside a
 * .svelte file is parsed as the component's own style block. */
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
	doc.querySelectorAll('a, area, form, button, input').forEach((el) => {
		for (const attr of ['href', 'action', 'formaction', 'ping', 'target', 'download']) {
			el.removeAttribute(attr)
		}
	})
	const style = doc.createElement('style')
	style.textContent = HIGHLIGHT_CSS
	head.appendChild(style)
	return `<!DOCTYPE html>${doc.documentElement.outerHTML}`
}

function textOf(el: Element | null | undefined, max = 40): string {
	const text = (el?.textContent ?? '').replace(/\s+/g, ' ').trim()
	return text.length > max ? `${text.slice(0, max)}…` : text
}

/** Short human name of an element, preferring what a user would call it
 * (its label / accessible name) over its markup. */
export function describeElement(el: Element): string {
	const tag = el.tagName.toLowerCase()
	const role = tag === 'input' ? `input[${(el.getAttribute('type') ?? 'text').toLowerCase()}]` : tag
	const labels = (el as HTMLInputElement).labels
	const name =
		el.getAttribute('aria-label') ||
		textOf(labels?.[0]) ||
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
			return `${value === 'checked' ? 'Checked' : 'Unchecked'} ${target}`
		case 'submit':
			return `Submitted ${target}`
		case 'key':
			return `Pressed ${value ?? 'key'} in ${target}`
		case 'navigate':
			return value ? `Navigated to ${value}` : 'Reloaded the app'
	}
}
