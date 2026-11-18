import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('esm-env', async (importOriginal) => ({
	...(await importOriginal<typeof import('esm-env')>()),
	BROWSER: true
}))
vi.mock('$lib/userScopedStorage', () => ({ scopedKey: (base: string) => `${base}::u` }))

const { recordHandEdit, removeHandEdit, sessionHandEdits } = await import(
	'./sessionHandEdits.svelte'
)

beforeEach(() => localStorage.clear())

describe('session hand edits', () => {
	it("leaves another tab's entries alone when writing its own", () => {
		recordHandEdit('s1', 'script', 'u/a/one')
		// Another tab records its own entry directly in the shared store.
		localStorage.setItem('windmill-session-hand-edit::u:s1:flow:u/a/two', '1')

		recordHandEdit('s1', 'raw_app', 'u/a/three')
		removeHandEdit('s1', 'script', 'u/a/one')

		expect([...sessionHandEdits('s1')].sort()).toEqual(['flow:u/a/two', 'raw_app:u/a/three'])
		expect(sessionHandEdits('s2').size).toBe(0)
	})
})
