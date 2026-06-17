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

/** Format a name as a mention token — bracket it when it contains whitespace, escaping
 *  any `\` and `]` so the bracketed form round-trips. */
export function formatMention(name: string): string {
	return /\s/.test(name) ? `@[${name.replace(/[\\\]]/g, '\\$&')}]` : `@${name}`
}
