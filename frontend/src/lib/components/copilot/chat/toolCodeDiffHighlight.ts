import hljs from 'highlight.js/lib/core'

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
	const source = truncated ? sourceLines.slice(0, lineLimit).join('\n') : code
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
