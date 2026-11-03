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
export async function printArtifact(name: string, content: string): Promise<void> {
	const iframe = document.createElement('iframe')
	iframe.setAttribute('aria-hidden', 'true')
	iframe.style.cssText = 'position:fixed;right:0;bottom:0;width:0;height:0;border:0'
	document.body.appendChild(iframe)

	const doc = iframe.contentDocument
	const win = iframe.contentWindow
	if (!doc || !win) {
		iframe.remove()
		return
	}
	// The print dialog proposes the title as the PDF's file name.
	doc.title = name
	for (const node of document.querySelectorAll('link[rel="stylesheet"], style')) {
		doc.head.appendChild(node.cloneNode(true))
	}
	const page = doc.createElement('style')
	// Without exact color adjust, browsers drop code-block and callout backgrounds. The break
	// rules keep a code block, diagram, callout or table row whole and a heading with what follows.
	page.textContent = `
		@page { margin: 16mm }
		html, body { background: white }
		body { margin: 0; print-color-adjust: exact; -webkit-print-color-adjust: exact }
		pre, svg, blockquote, tr, .markdown-alert { break-inside: avoid }
		h1, h2, h3, h4, h5, h6 { break-after: avoid }
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
	await diagramsDrawn(doc, content)
	await doc.fonts?.ready

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

/** Mermaid draws asynchronously; printing before it lands puts the diagram source in the PDF.
 * A diagram that fails to parse never draws, hence the cap. */
async function diagramsDrawn(doc: Document, content: string): Promise<void> {
	const expected = content.match(/^\s*(```|~~~)\s*mermaid\b/gm)?.length ?? 0
	const deadline = Date.now() + 10_000
	while (doc.querySelectorAll('svg[id^="mermaid-"]').length < expected && Date.now() < deadline) {
		await new Promise((resolve) => setTimeout(resolve, 100))
	}
}
