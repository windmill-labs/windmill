import hljs from 'highlight.js/lib/core'
import { escapeHtml } from '$lib/utils'

// highlight.js runs on the UI thread at roughly 15 ms per 1,000 lines (50 KB), and must see the
// whole source for multi-line strings and comments to scope correctly. Larger sides, including
// minified one-line sources, render as plain text with each line cut at MAX_PLAIN_LINE_CHARS.
const MAX_HIGHLIGHTED_LINES = 2_000
const MAX_HIGHLIGHTED_CHARS = 100_000
const MAX_PLAIN_LINE_CHARS = 10_000

// Highlights the whole source before splitting it, so a line inside a multi-line string or
// comment keeps that scope. Only the first `lineLimit` lines are highlighted: a streamed source
// grows on every delta while the preview shows a bounded prefix.
export function highlightedSourceLines(
	code: string,
	languageName: string,
	lineLimit = Infinity
): string[] {
	if (code === '' || lineLimit <= 0) return []
	const sourceLines = code.split('\n')
	const truncated = sourceLines.length > lineLimit
	const shownLines = truncated ? sourceLines.slice(0, lineLimit) : sourceLines
	const source = truncated ? shownLines.join('\n') : code
	if (shownLines.length > MAX_HIGHLIGHTED_LINES || source.length > MAX_HIGHLIGHTED_CHARS) {
		const plain = shownLines.map((line) =>
			escapeHtml(
				line.length > MAX_PLAIN_LINE_CHARS ? line.slice(0, MAX_PLAIN_LINE_CHARS) + '…' : line
			)
		)
		if (!truncated && code.endsWith('\n')) plain.pop()
		return plain
	}
	const highlighted = hljs.highlight(source, { language: languageName }).value
	return splitHighlightedLines(highlighted, !truncated && code.endsWith('\n'))
}

// Returns one entry per source line, closing the spans still open at each line end and
// reopening them on the next line so every entry is balanced HTML.
export function splitHighlightedLines(highlighted: string, hasFinalNewline: boolean): string[] {
	const result: string[] = []
	const openTags: string[] = []
	let line = ''

	for (const token of highlighted.split(/(<[^>]+>|\n)/)) {
		if (token === '\n') {
			result.push(line + '</span>'.repeat(openTags.length))
			line = openTags.join('')
		} else {
			line += token
			if (token.startsWith('<span')) openTags.push(token)
			else if (token === '</span>') openTags.pop()
		}
	}

	if (!hasFinalNewline) result.push(line)
	return result
}
