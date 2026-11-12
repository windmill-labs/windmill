import { mount, unmount, tick } from 'svelte'
import ArtifactPrintBody from './ArtifactPrintBody.svelte'

/**
 * Opens the browser's print dialog on a markdown artifact, where "Save as PDF" writes the file.
 * The browser's print engine keeps the text selectable and the links live, which a
 * canvas-rendered PDF would not.
 *
 * The document is rendered into a hidden iframe with the viewer's renderer rather than printing
 * this page: the app's own print would include the whole session layout. The iframe copies
 * this page's stylesheets, and its <html> carries no `dark` class, so the PDF always uses the
 * light theme.
 */
// Set while the iframe is being prepared, so a second click before the dialog opens does not
// stack a second print behind the first.
let preparing = false

export async function printArtifact(name: string, content: string): Promise<void> {
	if (preparing) return
	preparing = true
	try {
		await preparePrint(name, content)
	} finally {
		preparing = false
	}
}

const PAGE_MARGIN_MM = 16
// The frame is laid out as wide as an A4 page's text, the narrowest of the common paper sizes,
// so what it measures holds in print.
const PRINT_WIDTH_MM = 210 - 2 * PAGE_MARGIN_MM

async function preparePrint(name: string, content: string): Promise<void> {
	const iframe = document.createElement('iframe')
	iframe.setAttribute('aria-hidden', 'true')
	iframe.style.cssText = `position:fixed;right:0;bottom:0;width:${PRINT_WIDTH_MM}mm;height:0;border:0`
	document.body.appendChild(iframe)

	const doc = iframe.contentDocument
	const win = iframe.contentWindow
	if (!doc || !win) {
		iframe.remove()
		return
	}
	// The print dialog proposes the title as the PDF's file name.
	doc.title = name
	for (const node of Array.from(document.querySelectorAll('link[rel="stylesheet"], style'))) {
		doc.head.appendChild(node.cloneNode(true))
	}
	const page = doc.createElement('style')
	// Without exact color adjust, browsers drop code-block and callout backgrounds. The break
	// rules keep a code block, diagram, callout or table row whole and a heading with what follows.
	// Code blocks scroll on screen; paper cannot, so they wrap and lose their height cap.
	page.textContent = `
		@page { margin: ${PAGE_MARGIN_MM}mm }
		html, body { background: white }
		body { margin: 0; print-color-adjust: exact; -webkit-print-color-adjust: exact }
		pre, svg, blockquote, tr, .markdown-alert { break-inside: avoid }
		h1, h2, h3, h4, h5, h6 { break-after: avoid }
		.not-prose, .not-prose * { overflow: visible !important; max-height: none !important }
		.not-prose pre, .not-prose code { white-space: pre-wrap !important; overflow-wrap: anywhere }
	`
	doc.head.appendChild(page)

	// Printing before the stylesheets load gives an unstyled PDF; the cap keeps a link that
	// never settles from swallowing the click.
	const stylesheets = Array.from(
		doc.head.querySelectorAll('link[rel="stylesheet"]'),
		(link) =>
			new Promise<void>((resolve) => {
				link.addEventListener('load', () => resolve(), { once: true })
				link.addEventListener('error', () => resolve(), { once: true })
			})
	)
	await Promise.race([
		Promise.all(stylesheets),
		new Promise((resolve) => setTimeout(resolve, 3000))
	])

	const component = mount(ArtifactPrintBody, { target: doc.body, props: { content } })
	await tick()
	await diagramsSettled(doc)
	await imagesSettled(doc)
	await doc.fonts?.ready
	fitTablesToPage(doc)

	const cleanup = () => {
		unmount(component)
		iframe.remove()
	}
	// print() blocks until the dialog closes in Chromium and Firefox, but returns at once in
	// Safari, so the iframe is removed on afterprint rather than after the call.
	win.addEventListener('afterprint', cleanup, { once: true })
	win.focus()
	win.print()
}

/** A table scrolls inside its frame on screen, but paper keeps only the columns that fit, so a
 * table wider than the page is zoomed down to the page's width. */
function fitTablesToPage(doc: Document): void {
	for (const table of Array.from(doc.querySelectorAll('table'))) {
		if (table.scrollWidth > table.clientWidth) {
			table.style.setProperty('zoom', String(table.clientWidth / table.scrollWidth))
		}
	}
}

/** The print dialog snapshots the document as it is, so an image still loading is missing from
 * the PDF. A broken image settles through `error`; the cap covers one that never answers. */
async function imagesSettled(doc: Document): Promise<void> {
	const pending = Array.from(doc.images)
		.filter((img) => !img.complete)
		.map(
			(img) =>
				new Promise<void>((resolve) => {
					img.addEventListener('load', () => resolve(), { once: true })
					img.addEventListener('error', () => resolve(), { once: true })
				})
		)
	await Promise.race([Promise.all(pending), new Promise((resolve) => setTimeout(resolve, 10_000))])
}

/** Mermaid draws asynchronously, and display blocks load their renderer on demand; printing
 * before they land puts their source in the PDF. One that fails to parse settles too (as its
 * source), so this waits only on renders still in flight; the cap covers one that never returns. */
async function diagramsSettled(doc: Document): Promise<void> {
	const deadline = Date.now() + 10_000
	const pending = '[data-mermaid-pending="true"], [data-display-pending="true"]'
	while (doc.querySelector(pending) && Date.now() < deadline) {
		await new Promise((resolve) => setTimeout(resolve, 100))
	}
}
