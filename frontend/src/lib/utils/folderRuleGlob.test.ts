import { describe, expect, it } from 'vitest'
import { folderRuleGlobMatches } from './folderRuleGlob'

// Expected matches are what `globset::Glob::new(glob).compile_matcher()` answers on the server.
const PATHS = ['x', 'jobs/a', 'jobs/a/b', 'a/b', 'a/x/b', 'a/c', 'abc', 'foo/z', 'bar/z/q', 'baz/z']
const CASES: [string, string[]][] = [
	['**', PATHS],
	['*', PATHS],
	['**/', PATHS],
	['jobs/*', ['jobs/a', 'jobs/a/b']],
	['a/**/b', ['a/b', 'a/x/b']],
	['**/x', ['x']],
	['a?c', ['a/c', 'abc']],
	['[!ab]*', ['x', 'jobs/a', 'jobs/a/b', 'foo/z']],
	['{foo,bar}/*', ['foo/z', 'bar/z/q']],
	['[unclosed', []]
]

describe('folderRuleGlobMatches', () => {
	it.each(CASES)('%s matches what the server matches', (glob, expected) => {
		expect(PATHS.filter((p) => folderRuleGlobMatches(glob, p))).toEqual(expected)
	})
})
