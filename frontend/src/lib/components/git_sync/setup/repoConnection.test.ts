import { describe, expect, test } from 'vitest'
import { parseRepoUrl, repoSlug, withToken } from './repoConnection'

describe('parseRepoUrl', () => {
	test('drops credentials, query and fragment from a pasted URL', () => {
		const url = parseRepoUrl(' https://someone:secret@github.com/acme/infra.git?utm=1#readme ')
		expect(url?.toString()).toBe('https://github.com/acme/infra.git')
	})

	test('refuses what cannot carry a token safely or name a repository', () => {
		// http would put the token on the wire, and the rest name no repository at all.
		expect(parseRepoUrl('http://github.com/acme/infra')).toBeUndefined()
		expect(parseRepoUrl('ssh://git@github.com/acme/infra')).toBeUndefined()
		expect(parseRepoUrl('github.com/acme/infra')).toBeUndefined()
		expect(parseRepoUrl('https://github.com/acme')).toBeUndefined()
	})
})

test('withToken encodes the token so a punctuated one survives the round trip', () => {
	const token = 'glpat-a/b@c:d'
	const embedded = withToken(parseRepoUrl('https://gitlab.com/acme/infra')!, token, 'gitlab')
	expect(embedded).toBe(`https://oauth2:${encodeURIComponent(token)}@gitlab.com/acme/infra`)
	expect(new URL(embedded).password).toBe(encodeURIComponent(token))
})

test('repoSlug names the resource after the repository', () => {
	expect(repoSlug('https://github.com/acme/My-Repo.git')).toBe('my_repo')
	expect(repoSlug('https://gitlab.com/group/sub/project/')).toBe('project')
})
