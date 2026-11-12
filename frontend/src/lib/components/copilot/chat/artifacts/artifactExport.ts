import { unified } from 'unified'
import remarkParse from 'remark-parse'
import remarkRehype from 'remark-rehype'
import rehypeStringify from 'rehype-stringify'
import tokensFile from '$lib/assets/tokens/tokens.json'
import { lightModeName } from '$lib/assets/tokens/colorTokensConfig'
import { randomUUID } from '$lib/utils/uuid'
import { blobToDataUrl } from '../imageUtils'
import { mermaidConfig } from '../script/mermaidConfig'
import { artifactPlugins } from './artifactPlugins'
import documentCss from './artifactDocument.css?inline'
import interUrl from './fonts/inter-latin.woff2?url'
import interItalicUrl from './fonts/inter-italic-latin.woff2?url'
import jetbrainsMonoUrl from '@fontsource-variable/jetbrains-mono/files/jetbrains-mono-latin-wght-normal.woff2?url'

// Built from `artifactPlugins` the way svelte-exmarkdown builds its own processor, so the
// export goes through the same sanitize stage as the in-app render. Its renderers (the
// highlighted code block, the link pill) are Svelte components and do not apply here.
const processor = unified()
	.use(remarkParse)
	.use(artifactPlugins.flatMap((p) => (p.remarkPlugin ? [p.remarkPlugin] : [])))
	.use(remarkRehype, { allowDangerousHtml: true })
	.use(artifactPlugins.flatMap((p) => (p.rehypePlugin ? [p.rehypePlugin] : [])))
	.use(rehypeStringify)

/** The tokens `artifactDocument.css` reads, as the rgb triplets the app's own variables hold. */
const DOCUMENT_TOKENS = [
	'text-primary',
	'text-emphasis',
	'text-secondary',
	'text-accent',
	'border-light',
	'border-normal',
	'surface-secondary'
] as const

function hexToTriplet(hex: string): string {
	const n = (i: number) => parseInt(hex.slice(i, i + 2), 16)
	const rgb = `${n(1)} ${n(3)} ${n(5)}`
	return hex.length === 9 ? `${rgb} / ${(n(7) / 255).toFixed(2)}` : rgb
}

const lightTokens = tokensFile.tokens[lightModeName]
const tokenCss = DOCUMENT_TOKENS.map((t) => `--color-${t}:${hexToTriplet(lightTokens[t])};`).join(
	''
)

const pageCss = `
:root{${tokenCss}color-scheme:light}
html,body{margin:0;background:#fff}
.wm-page{padding:72px 32px 96px}
@page{margin:16mm 14mm}
@media print{.wm-page{padding:0}}
`

function escapeHtml(s: string): string {
	return s.replace(
		/[&<>"']/g,
		(c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]!
	)
}

export function renderArtifactHtml(content: string): string {
	return String(processor.processSync(content))
}

interface EmbeddedFace {
	family: string
	style: 'normal' | 'italic'
	weight: string
	url: string
	/** Whether the rendered document can set text in this face. */
	needed: (html: string) => boolean
}

/**
 * The faces `artifactDocument.css` names, in Latin subsets: text in another script falls back
 * to the system fonts of each stack. The Inter files are cut from the upstream release, because
 * the Google and Fontsource builds drop most of its OpenType features: fontTools `subset
 * --layout-features='*' --name-IDs='*'` over Fontsource's Latin range plus arrows, ≈ ≠ ≤ ≥ and ✓.
 */
const EMBEDDED_FACES: EmbeddedFace[] = [
	{ family: 'Inter', style: 'normal', weight: '100 900', url: interUrl, needed: () => true },
	{
		family: 'Inter',
		style: 'italic',
		weight: '100 900',
		url: interItalicUrl,
		// Quotes are set in italic too.
		needed: (html) => /<(em|i|blockquote)\b/.test(html)
	},
	{
		family: 'JetBrains Mono Variable',
		style: 'normal',
		weight: '100 800',
		url: jetbrainsMonoUrl,
		needed: (html) => /<(code|kbd|pre)\b/.test(html)
	}
]

const fontDataUrls = new Map<string, Promise<string | undefined>>()

/** The font file as a data URL, or undefined when it cannot be fetched. */
function fontDataUrl(url: string): Promise<string | undefined> {
	let dataUrl = fontDataUrls.get(url)
	if (!dataUrl) {
		dataUrl = fetch(url)
			.then(async (res) => {
				if (!res.ok) return undefined
				return blobToDataUrl(new Blob([await res.arrayBuffer()], { type: 'font/woff2' }))
			})
			.catch(() => undefined)
		fontDataUrls.set(url, dataUrl)
		// A failure is not kept, so the next export tries again.
		dataUrl.then((d) => {
			if (!d) fontDataUrls.delete(url)
		})
	}
	return dataUrl
}

/**
 * `@font-face` rules carrying the fonts themselves, for the faces rendered `html` can set text
 * in, or for all of them without `html`. A face that cannot be fetched is left out, and its
 * text falls back to the next font of the stack.
 */
export async function artifactFontCss(html?: string): Promise<string> {
	const faces = EMBEDDED_FACES.filter((f) => html === undefined || f.needed(html))
	const rules = await Promise.all(
		faces.map(async ({ family, style, weight, url }) => {
			const src = await fontDataUrl(url)
			return src
				? `@font-face{font-family:'${family}';font-style:${style};font-weight:${weight};src:url(${src}) format('woff2')}`
				: ''
		})
	)
	return rules.filter(Boolean).join('\n')
}

export interface ArtifactHtmlOptions {
	/** Applied after the document's own stylesheet. */
	css?: string
	/** Stylesheets loaded first, for web fonts that `css` names. */
	fontLinks?: string[]
	/** `@font-face` rules, as `artifactFontCss` builds them. */
	fontCss?: string
}

function htmlPage(
	name: string,
	html: string,
	{ css = '', fontLinks = [], fontCss = '' }: ArtifactHtmlOptions
): string {
	const links = fontLinks.map((href) => `<link rel="stylesheet" href="${escapeHtml(href)}">`)
	return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${escapeHtml(name)}</title>
${links.join('\n')}
<style>${pageCss}${documentCss}${css}</style>
${fontCss && `<style>${fontCss}</style>`}
</head>
<body>
<div class="wm-page wm-doc-frame">
<main class="wm-doc">
${html}
</main>
</div>
</body>
</html>
`
}

/** A standalone page, which needs no network and no Windmill unless `fontLinks` are given. */
export function artifactHtmlDocument(
	name: string,
	content: string,
	options: ArtifactHtmlOptions = {}
): string {
	return htmlPage(name, renderArtifactHtml(content), options)
}

/**
 * `html` with its ```mermaid fences drawn, light as in the PDF. Only surfaces that already draw
 * diagrams offer the download, so this runs the same mermaid under the same settings.
 */
async function drawDiagrams(html: string): Promise<string> {
	if (!html.includes('language-mermaid')) return html
	// Parsed and built in the template's inert document: markup set on an element of the page
	// itself loads its images and runs their handlers, even while detached.
	const template = document.createElement('template')
	template.innerHTML = html
	const fences = Array.from(template.content.querySelectorAll('pre > code.language-mermaid'))
	if (fences.length === 0) return html
	const mermaid = (await import('mermaid')).default
	for (const code of fences) {
		const source = code.textContent ?? ''
		try {
			mermaid.initialize(mermaidConfig(false))
			let svg = (await mermaid.render(`mermaid-${randomUUID()}`, source)).svg
			// Labels are sized from the glyphs' metrics: a font still loading would cut them short.
			if (document.fonts?.status === 'loading') {
				await document.fonts.ready
				svg = (await mermaid.render(`mermaid-${randomUUID()}`, source)).svg
			}
			const figure = template.content.ownerDocument.createElement('figure')
			figure.className = 'wm-diagram'
			figure.innerHTML = svg
			code.parentElement!.replaceWith(figure)
		} catch {
			// Not a diagram mermaid can parse: the fence stays as its source.
		}
	}
	return template.innerHTML
}

/** `html` with its ```wm-display fences drawn as images and tables. */
async function drawDisplays(html: string): Promise<string> {
	if (!html.includes('language-wm-display')) return html
	// Built in the template's inert document, like the diagrams.
	const template = document.createElement('template')
	template.innerHTML = html
	const { drawDisplayFences } = await import('$lib/components/display/displayExport')
	await drawDisplayFences(template.content)
	return template.innerHTML
}

/** The page a download gets: `artifactHtmlDocument` with its diagrams drawn and fonts embedded. */
export async function artifactHtmlFile(name: string, content: string): Promise<string> {
	const html = await drawDisplays(await drawDiagrams(renderArtifactHtml(content)))
	return htmlPage(name, html, { fontCss: await artifactFontCss(html) })
}
