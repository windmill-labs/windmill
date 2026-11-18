import type { Plugin } from 'svelte-exmarkdown'
import type { Element, ElementContent, Root } from 'hast'
import { visit } from 'unist-util-visit'
import { markdownPlugins } from '$lib/components/markdownPlugins'

/** About as many characters as the 12em minimum width of a long cell (`artifactDocument.css`). */
const SHORT_CELL_CHARS = 24

function textOf(node: ElementContent): string {
	if (node.type === 'text') return node.value
	if (node.type === 'element') return node.children.map(textOf).join('')
	return ''
}

function addClass(node: Element, name: string) {
	const className = node.properties.className
	node.properties.className = [...(Array.isArray(className) ? className : []), name]
}

/**
 * Marks each table cell short or long for the document stylesheet. It runs after the sanitize
 * stage, which would strip the classes.
 */
function rehypeTableCells() {
	return (tree: Root) => {
		visit(tree, 'element', (node: Element) => {
			if (node.tagName !== 'td' && node.tagName !== 'th') return
			addClass(node, textOf(node).length > SHORT_CELL_CHARS ? 'wm-cell-long' : 'wm-cell-short')
		})
	}
}

/**
 * Marks every section heading unnumbered when one is numbered 1 (`## 1. Goals`, `## 1️⃣ Goals`):
 * the stylesheet numbers sections, and a document that numbers its own would show both. A
 * heading that only starts with a 1, as `## 19th century` or `## 1.5 release` do, does not count.
 */
function rehypeSectionNumbers() {
	return (tree: Root) => {
		const headings: Element[] = []
		visit(tree, 'element', (node: Element) => {
			if (node.tagName === 'h2') headings.push(node)
		})
		if (!headings.some((h) => /^\s*1(?:[.):]|️?⃣)?(?:\s|$)/.test(textOf(h)))) return
		for (const h of headings) addClass(h, 'wm-unnumbered')
	}
}

/** The shared chain plus the markup the artifact document's stylesheet reads. */
export const artifactPlugins: Plugin[] = [
	...markdownPlugins,
	{ rehypePlugin: [rehypeTableCells] },
	{ rehypePlugin: [rehypeSectionNumbers] }
]
