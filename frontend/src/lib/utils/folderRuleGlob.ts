/**
 * Whether a folder `default_permissioned_as` glob matches a path relative to its folder, with
 * the semantics the server applies (`globset::Glob` defaults): `*` and `?` also match `/`,
 * `**` spans any depth, `{a,b}` alternates, `[!x]` negates, and an invalid glob matches nothing.
 * The editors send the rule they preview, so a matcher that disagrees with the server stores
 * an identity other than the one the folder rule would have given.
 */
export function folderRuleGlobMatches(glob: string, relativePath: string): boolean {
	const source = globToRegexSource(glob)
	return source !== undefined && new RegExp(`^${source}$`, 's').test(relativePath)
}

function globToRegexSource(glob: string): string | undefined {
	let re = ''
	for (let i = 0; i < glob.length; i++) {
		const c = glob[i]
		if (c === '*') {
			if (glob[i + 1] !== '*') {
				re += '.*'
				continue
			}
			const atSegmentStart = i === 0 || glob[i - 1] === '/'
			i++
			if (atSegmentStart && glob[i + 1] === '/' && i + 2 < glob.length) {
				re += '(?:.*/)?'
				i++
			} else {
				re += '.*'
				if (glob[i + 1] === '/') i++
			}
		} else if (c === '?') {
			re += '.'
		} else if (c === '[') {
			const end = glob.indexOf(']', i + 2)
			if (end < 0) return undefined
			let body = glob.slice(i + 1, end).replace(/[\\^]/g, '\\$&')
			if (body.startsWith('!')) body = '^' + body.slice(1)
			re += `[${body}]`
			i = end
		} else if (c === '{') {
			const end = glob.indexOf('}', i)
			if (end < 0) return undefined
			const alternatives = glob
				.slice(i + 1, end)
				.split(',')
				.map(globToRegexSource)
			if (alternatives.some((a) => a === undefined)) return undefined
			re += `(?:${alternatives.join('|')})`
			i = end
		} else if (c === '\\' && i + 1 < glob.length) {
			re += escapeRegex(glob[++i])
		} else {
			re += escapeRegex(c)
		}
	}
	return re
}

function escapeRegex(c: string): string {
	return c.replace(/[.*+?^${}()|[\]\\/]/g, '\\$&')
}
