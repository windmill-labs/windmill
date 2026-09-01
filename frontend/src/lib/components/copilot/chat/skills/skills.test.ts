import { beforeEach, describe, expect, it, vi } from 'vitest'

const { session } = vi.hoisted(() => ({
	session: { email: 'first@windmill.dev' } as { email?: string }
}))

vi.mock('$lib/stores', () => ({
	// Read at call time, so a test can switch accounts the way a logout does.
	userStore: { subscribe: (run: (v: unknown) => void) => (run({ ...session }), () => {}) }
}))

import { enabledSkillPaths, isSkillEnabled, setSkillEnabled } from './enabledSkills'
import { ambiguousSkillNames, truncateChars, truncateForPrompt } from './skillResources'

describe('enabledSkills', () => {
	beforeEach(() => {
		localStorage.clear()
		session.email = 'first@windmill.dev'
	})

	it('keeps the selection separate per workspace', () => {
		setSkillEnabled('ws_a', 'u/me/deploy', true)
		expect(isSkillEnabled('ws_a', 'u/me/deploy')).toBe(true)
		expect(isSkillEnabled('ws_b', 'u/me/deploy')).toBe(false)
	})

	it('does not hand the next account the previous one’s selection', () => {
		setSkillEnabled('ws_a', 'u/me/deploy', true)
		session.email = 'second@windmill.dev'
		expect(enabledSkillPaths('ws_a')).toEqual([])
	})

	it('reports failure when there is no account to record the choice against', () => {
		session.email = undefined
		expect(setSkillEnabled('ws_a', 'u/me/deploy', true)).toBe(false)
		expect(enabledSkillPaths('ws_a')).toEqual([])
	})
})

describe('skill names', () => {
	it('flags a basename two folders both use, so /name is not resolved by chance', () => {
		const ambiguous = ambiguousSkillNames([
			{ name: 'deploy' },
			{ name: 'deploy' },
			{ name: 'release' }
		])
		expect([...ambiguous]).toEqual(['deploy'])
	})
})

describe('prompt truncation', () => {
	// The two caps are stated in different units, and using one truncator for both
	// either lets three times the payload through or cuts a legal value to a third.
	it('bounds a skill body by utf-8 bytes, not code units', () => {
		const body = '漢'.repeat(100) // 300 bytes
		expect(truncateForPrompt(body, 3000)).toBe(body)
		const cut = truncateForPrompt(body, 30)
		expect(new TextEncoder().encode(cut.replace('… [truncated]', '')).byteLength).toBeLessThanOrEqual(30)
		expect(cut).toContain('[truncated]')
		// A byte-aligned cut must not leave a broken code point behind.
		expect(cut).not.toContain('\ufffd')
	})

	it('bounds a description by code points, so a CJK one is not cut to a third', () => {
		const description = '漢'.repeat(100)
		expect(truncateChars(description, 100)).toBe(description)
		expect([...truncateChars(description, 10)].slice(0, 10).join('')).toBe('漢'.repeat(10))
	})
})
