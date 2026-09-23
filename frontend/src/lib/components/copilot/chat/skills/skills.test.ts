import { beforeEach, describe, expect, it, vi } from 'vitest'

const { session } = vi.hoisted(() => ({
	session: { email: 'first@windmill.dev' } as { email?: string }
}))

vi.mock('$lib/stores', () => ({
	// Read at call time, so a test can switch accounts the way a logout does.
	userStore: { subscribe: (run: (v: unknown) => void) => (run({ ...session }), () => {}) }
}))

import { isMcpEnabled } from '$lib/components/mcp/enabledServers'
import { isSkillEnabled, setSkillEnabled } from './enabledSkills'
import { ambiguousSkillNames, truncateChars, truncateForPrompt } from './skillResources'
import { buildSkillTree, countSkills, skillFolderPaths, visibleEntries } from './skillTree'

describe('enabledSkills', () => {
	beforeEach(() => {
		localStorage.clear()
		session.email = 'first@windmill.dev'
	})

	it('keeps a skill turned off separate per workspace', () => {
		setSkillEnabled('ws_a', 'u/me/deploy', false)
		expect(isSkillEnabled('ws_a', 'u/me/deploy')).toBe(false)
		expect(isSkillEnabled('ws_b', 'u/me/deploy')).toBe(true)
	})

	it('does not hand the next account the previous one’s choice', () => {
		setSkillEnabled('ws_a', 'u/me/deploy', false)
		session.email = 'second@windmill.dev'
		expect(isSkillEnabled('ws_a', 'u/me/deploy')).toBe(true)
	})

	it('reports failure when there is no account to record the choice against', () => {
		session.email = undefined
		expect(setSkillEnabled('ws_a', 'u/me/deploy', false)).toBe(false)
	})
})

// Storage written before skills defaulted to on holds an array of the paths that
// were on. Nothing rewrites it, so reading it wrong is what would silently move
// somebody's choices — in either direction, for either default.
describe('choices stored under the older shape', () => {
	const scope = 'ws_a:first@windmill.dev'

	beforeEach(() => {
		localStorage.clear()
		session.email = 'first@windmill.dev'
	})

	it('keeps a skill that was turned on, and defaults the rest to on', () => {
		localStorage.setItem('wm_skills_enabled', JSON.stringify({ [scope]: ['u/me/deploy'] }))
		expect(isSkillEnabled('ws_a', 'u/me/deploy')).toBe(true)
		expect(isSkillEnabled('ws_a', 'u/me/never-picked')).toBe(true)
	})

	it('leaves an MCP server that was never turned on off', () => {
		localStorage.setItem('wm_mcp_enabled', JSON.stringify({ [scope]: ['u/me/github'] }))
		expect(isMcpEnabled('ws_a', 'u/me/github')).toBe(true)
		expect(isMcpEnabled('ws_a', 'u/me/other')).toBe(false)
	})

	it('records an off decision beside the entries already there', () => {
		localStorage.setItem('wm_skills_enabled', JSON.stringify({ [scope]: ['u/me/deploy'] }))
		setSkillEnabled('ws_a', 'u/me/review', false)
		expect(isSkillEnabled('ws_a', 'u/me/review')).toBe(false)
		expect(isSkillEnabled('ws_a', 'u/me/deploy')).toBe(true)
	})

	it('drops a path put back to the default instead of storing it', () => {
		setSkillEnabled('ws_a', 'u/me/review', false)
		setSkillEnabled('ws_a', 'u/me/review', true)
		expect(JSON.parse(localStorage.getItem('wm_skills_enabled') ?? '{}')[scope]).toEqual({})
	})
})

describe('skill tree', () => {
	const skills = [
		{ path: 'f/skills/deploy/rollback', name: 'rollback' },
		{ path: 'f/skills/onboarding', name: 'onboarding' },
		{ path: 'u/admin/release-notes', name: 'release-notes' }
	]

	it('nests each path segment under its owner, own folder first', () => {
		const tree = buildSkillTree(skills)
		expect(tree.map((n) => n.path)).toEqual(['u/admin', 'f/skills'])
		const shared = tree[1]
		expect(shared.skills.map((s) => s.name)).toEqual(['onboarding'])
		// A skill two levels down gets its folder, rather than being flattened into
		// the owner's own rows where the path it came from is lost.
		expect(shared.children.map((c) => c.path)).toEqual(['f/skills/deploy'])
		expect(shared.children[0].skills.map((s) => s.name)).toEqual(['rollback'])
		expect(countSkills(shared)).toBe(2)
	})

	// The keyboard walks this list while the markup renders the forest recursively. If
	// the two disagree, Down lands on a row that is not the one lit.
	it('lists what is on screen, and holds back what a collapsed folder hides', () => {
		const tree = buildSkillTree(skills)
		expect(visibleEntries(tree, () => false).map((e) => e.key)).toEqual([
			'folder:u/admin',
			'skill:u/admin/release-notes',
			'folder:f/skills',
			'folder:f/skills/deploy',
			'skill:f/skills/deploy/rollback',
			'skill:f/skills/onboarding'
		])
		expect(visibleEntries(tree, (path) => path === 'f/skills').map((e) => e.key)).toEqual([
			'folder:u/admin',
			'skill:u/admin/release-notes',
			'folder:f/skills'
		])
	})

	// What the list uses to choose between the tree and a flat list.
	it('counts the folders holding the skills, not the skills', () => {
		expect(skillFolderPaths(skills).size).toBe(3)
		expect(skillFolderPaths([skills[1], { path: 'f/skills/deploy', name: 'deploy' }]).size).toBe(1)
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
		expect(
			new TextEncoder().encode(cut.replace('… [truncated]', '')).byteLength
		).toBeLessThanOrEqual(30)
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
