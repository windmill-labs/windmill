import { describe, it, expect } from 'vitest'
import { versionThisDeployWrote } from './appDeploy.svelte'

describe('versionThisDeployWrote', () => {
	it('takes the head this deploy wrote', () => {
		expect(versionThisDeployWrote([{ version: 7, created_by: 'alice' }], 'alice')).toBe(7)
	})

	it('finds its own version under a deploy that landed on top', () => {
		// Pinning the head would make Bob's version the base of content it never
		// contained, and the next deploy would find base === head and overwrite it.
		expect(
			versionThisDeployWrote(
				[
					{ version: 8, created_by: 'bob' },
					{ version: 7, created_by: 'alice' }
				],
				'alice'
			)
		).toBe(7)
	})

	it('claims nothing it cannot attribute', () => {
		expect(versionThisDeployWrote([{ version: 8, created_by: 'bob' }], 'alice')).toBe(undefined)
		expect(versionThisDeployWrote([], 'alice')).toBe(undefined)
		expect(versionThisDeployWrote([{ version: 7, created_by: 'alice' }], undefined)).toBe(undefined)
	})
})
