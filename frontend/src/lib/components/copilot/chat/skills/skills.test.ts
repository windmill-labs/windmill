import { beforeEach, describe, expect, it, vi } from 'vitest'

const { session } = vi.hoisted(() => ({
	session: { email: 'first@windmill.dev' } as { email?: string }
}))

vi.mock('$lib/stores', () => ({
	// Read at call time, so a test can switch accounts the way a logout does.
	userStore: { subscribe: (run: (v: unknown) => void) => (run({ ...session }), () => {}) }
}))

import { enabledSkillPaths, isSkillEnabled, setSkillEnabled } from './enabledSkills'
import { ambiguousSkillNames } from './skillResources'

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
