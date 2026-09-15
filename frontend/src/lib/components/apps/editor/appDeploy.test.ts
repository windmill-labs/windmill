import { describe, it, expect } from 'vitest'
import { versionThisDeployWrote } from './appDeploy.svelte'

describe('versionThisDeployWrote', () => {
	it('takes the head this deploy wrote', () => {
		expect(versionThisDeployWrote([{ version: 7, created_by: 'alice' }], 'alice')).toBe(7)
	})

	it('claims nothing when another deploy landed on top', () => {
		// The base would otherwise be a version this content never contained, and the
		// next deploy would find base === head and overwrite it unwarned.
		expect(versionThisDeployWrote([{ version: 8, created_by: 'bob' }], 'alice')).toBe(undefined)
		expect(versionThisDeployWrote([], 'alice')).toBe(undefined)
		expect(versionThisDeployWrote([{ version: 7, created_by: 'alice' }], undefined)).toBe(undefined)
	})
})
