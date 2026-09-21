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

function isWordChar(char: string | undefined): boolean {
	return char !== undefined && /[\p{L}\p{N}_]/u.test(char)
}

export function isStandaloneMentionAt(text: string, token: string, index: number): boolean {
	const start = index
	const end = start + token.length
	return !isWordChar(text[start - 1]) && !isWordChar(text[end])
}

export function isStandaloneMention(text: string, match: RegExpMatchArray): boolean {
	if (match.index === undefined) return false
	return isStandaloneMentionAt(text, match[0], match.index)
}

export function hasMention(text: string, title: string): boolean {
	return [...text.matchAll(MENTION_RE)].some(
		(m) => mentionTitle(m[0]) === title && isStandaloneMention(text, m)
	)
}

export function mentionTitlesInText(text: string): Set<string> {
	const out = new Set<string>()
	for (const m of text.matchAll(MENTION_RE)) {
		if (isStandaloneMention(text, m)) out.add(mentionTitle(m[0]))
	}
	return out
}

export function removeMentionFromText(text: string, title: string): string {
	let out = ''
	let last = 0
	for (const m of text.matchAll(MENTION_RE)) {
		if (m.index === undefined || mentionTitle(m[0]) !== title) continue
		let start = m.index
		let end = m.index + m[0].length
		if (!isStandaloneMention(text, m)) continue
		const hasLead = start > 0 && /\s/.test(text[start - 1])
		const hasTrail = end < text.length && /\s/.test(text[end])
		if (hasLead && !hasTrail) start -= 1
		if (!hasLead && hasTrail) end += 1
		if (hasLead && hasTrail) end += 1
		out += text.slice(last, start)
		last = end
	}
	return out + text.slice(last)
}
