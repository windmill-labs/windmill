import { parseExpressionAt } from 'acorn'

/**
 * Template mode stores what the author typed as a JS template literal, so the text is spliced
 * between backticks. A backtick in the literal part has to be escaped or it ends the literal
 * early — but one inside a `${...}` must not be, since a backslash is a syntax error in
 * expression position and a nested template literal there is legitimate.
 *
 * Telling those apart means knowing where each `${...}` ends, which needs a real JS lexer:
 * regex literals, comments and nested templates all hide braces and backticks from anything
 * simpler. So rather than escaping selectively, ask the parser whether the text already reads as
 * one template literal. If it does, it needs no escaping at all; if it does not, escape every
 * backtick, which is what this did before nested templates were supported.
 *
 * Known limitation: a value mixing a bare literal backtick with a nested template cannot be
 * expressed either way, and gets the all-or-nothing fallback. Escaping the literal one by hand
 * makes the whole value parse and it is then kept verbatim.
 */
function isCompleteTemplateBody(text: string): boolean {
	const source = '`' + text + '`'
	try {
		const node = parseExpressionAt(source, 0, { ecmaVersion: 'latest' })
		// The type is what rejects a body that closes its own literal early: `` ` + evil() + ` ``
		// parses, but as a concatenation, and would evaluate the author's literal text. The span
		// rejects a body that stops short, like `` `a` x ``.
		return node.type === 'TemplateLiteral' && node.start === 0 && node.end === source.length
	} catch {
		return false
	}
}

/** Escape `text` so it can be wrapped in backticks and mean what the author typed. */
export function escapeTemplateBackticks(text: string): string {
	// No backtick means nothing to escape and nothing to decide, which is every ordinary value.
	if (!text.includes('`')) {
		return text
	}
	return isCompleteTemplateBody(text) ? text : text.replaceAll('`', '\\`')
}

/** Inverse of {@link escapeTemplateBackticks}, for turning an expression back into a template. */
export function unescapeTemplateBackticks(text: string): string {
	if (!text.includes('`')) {
		return text
	}
	const unescaped = text.replaceAll('\\`', '`')
	// The unescaped form is what the author typed when re-escaping reproduces the stored text,
	// and also whenever it reads as a template literal in its own right — which is how an
	// expression escaped by the old blanket rule comes back clean instead of showing backslashes
	// the author never typed. Otherwise keep the text: unescaping it would corrupt a backtick
	// that is escaped inside a nested template, where it belongs.
	if (escapeTemplateBackticks(unescaped) === text || isCompleteTemplateBody(unescaped)) {
		return unescaped
	}
	return text
}
