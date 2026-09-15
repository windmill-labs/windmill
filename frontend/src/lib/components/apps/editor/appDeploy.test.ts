import { describe, it, expect } from 'vitest'
import { versionThisDeployWrote } from './appDeploy.svelte'

const alice = (version: number) => ({ version, created_by: 'alice' })
const bob = (version: number) => ({ version, created_by: 'bob' })

describe('versionThisDeployWrote', () => {
	it('takes the entry this deploy appended to the head it read', () => {
		expect(versionThisDeployWrote([alice(7), bob(6)], 'alice', 6)).toBe(7)
	})

	it('finds its own version under a deploy that landed on top', () => {
		// Pinning the head would make Bob's version the base of content it never
		// contained, and the next deploy would find base === head and overwrite it.
		expect(versionThisDeployWrote([bob(8), alice(7), bob(6)], 'alice', 6)).toBe(7)
	})

	it('takes the first version of an app that had none', () => {
		expect(versionThisDeployWrote([alice(1)], 'alice', undefined)).toBe(1)
	})

	it('claims nothing it cannot tell apart from another deploy', () => {
		// The same account deploying from elsewhere is indistinguishable by author, so
		// the entry has to sit on the head this deploy read, and this one does not.
		expect(versionThisDeployWrote([alice(9), alice(7), bob(6)], 'alice', 6)).toBe(undefined)
		expect(versionThisDeployWrote([bob(8), bob(6)], 'alice', 6)).toBe(undefined)
		expect(versionThisDeployWrote([], 'alice', 6)).toBe(undefined)
		expect(versionThisDeployWrote([alice(7), bob(6)], undefined, 6)).toBe(undefined)
	})
})
