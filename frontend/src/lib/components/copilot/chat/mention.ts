/**
 * `@mention` formatting shared between the chat input (which inserts mentions) and the
 * textarea highlighter (which parses them) so the two never disagree.
 *
 * A simple name is inserted bare (`@app.ts`); a name containing whitespace is bracketed
 * (`@[my file.txt]`) so it's captured whole instead of truncating at the first space.
 */

/**
 * Matches a mention token: a bracketed `@[name with spaces]` (where `\]` and `\\` are
 * escaped, so a `]` inside the name doesn't end the token early) first, then a bare `@name`.
 */
export const MENTION_RE = /@\[(?:\\.|[^\]\\\r\n])*\]|@[\w/.\-\[\]]+/g

/** The title of a mention token (`@name` or `@[name]`), brackets stripped and unescaped. */
export function mentionTitle(token: string): string {
	if (token.startsWith('@[') && token.endsWith(']')) {
		return token.slice(2, -1).replace(/\\(.)/g, '$1')
	}
	return token.slice(1)
}

/** Chars the bare `@name` regex matches without truncating; anything else needs brackets. */
const BARE_SAFE = /^[\w/.\-]+$/

/** Format a name as a mention token. A bare `@name` only survives for simple names; anything
 *  with whitespace, HTML-sensitive chars (`< > &`), brackets, parens, etc. is bracketed (with
 *  `\` and `]` escaped) so the token is captured whole and round-trips through the parser. */
export function formatMention(name: string): string {
	return BARE_SAFE.test(name) ? `@${name}` : `@[${name.replace(/[\\\]]/g, '\\$&')}]`
}
