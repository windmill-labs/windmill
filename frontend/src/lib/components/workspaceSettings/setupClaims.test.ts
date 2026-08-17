import { describe, expect, it } from 'vitest'
import {
	anythingClaimed,
	claim,
	claimsFromJSON,
	claimsToJSON,
	noClaims,
	release,
	stillOurs
} from './setupClaims'

describe('stillOurs', () => {
	it('honours a claim whose object has not moved', () => {
		const claims = claim(noClaims, 'secret', 'f/team/db', 'alice')
		expect(stillOurs(claims, 'secret', 'f/team/db', 'alice')).toBe(true)
	})

	it('refuses when the object was last written by somebody else', () => {
		const claims = claim(noClaims, 'secret', 'f/team/db', 'alice')
		expect(stillOurs(claims, 'secret', 'f/team/db', 'bob')).toBe(false)
	})

	// Deleted and recreated between two attempts: something is there, it is not ours.
	it('refuses when the object is gone', () => {
		const claims = claim(noClaims, 'resource', 'f/team/db', 'alice')
		expect(stillOurs(claims, 'resource', 'f/team/db', undefined)).toBe(false)
	})

	it('refuses a path this run never claimed', () => {
		expect(stillOurs(noClaims, 'secret', 'f/team/db', 'alice')).toBe(false)
	})

	// The secret and the resource are separate objects at one path.
	it('keeps the two objects at one path apart', () => {
		const claims = claim(noClaims, 'secret', 'f/team/db', 'alice')
		expect(stillOurs(claims, 'secret', 'f/team/db', 'alice')).toBe(true)
		expect(stillOurs(claims, 'resource', 'f/team/db', 'alice')).toBe(false)
	})

	it('refuses a row repointed since it was written', () => {
		const claims = claim(noClaims, 'row', 'main', 'f/team/db')
		expect(stillOurs(claims, 'row', 'main', 'f/team/db')).toBe(true)
		expect(stillOurs(claims, 'row', 'main', 'someone-elses-db')).toBe(false)
	})
})

describe('claims as a set', () => {
	it('replaces the mark when the same object is claimed again', () => {
		let claims = claim(noClaims, 'row', 'main', 'first')
		claims = claim(claims, 'row', 'main', 'second')
		expect(claims).toHaveLength(1)
		expect(stillOurs(claims, 'row', 'main', 'second')).toBe(true)
	})

	it('gives a claim up so the name is free again', () => {
		const claims = release(claim(noClaims, 'row', 'main', 'x'), 'row', 'main')
		expect(anythingClaimed(claims)).toBe(false)
	})

	it('carries every claim across the redirect, whatever kinds are held', () => {
		let claims = claim(noClaims, 'secret', 'f/team/db', 'alice')
		claims = claim(claims, 'resource', 'f/team/db', 'alice')
		claims = claim(claims, 'row', 'main', 'f/team/db')
		const restored = claimsFromJSON(JSON.parse(JSON.stringify(claimsToJSON(claims))))
		expect(restored).toEqual(claims)
	})

	it('survives a payload that is not claims at all', () => {
		expect(claimsFromJSON(undefined)).toEqual(noClaims)
		expect(claimsFromJSON([{ kind: 'nonsense', path: 'p', mark: 'm' }])).toEqual(noClaims)
	})
})
