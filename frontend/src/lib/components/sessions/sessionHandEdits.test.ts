import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('esm-env', async (importOriginal) => ({
	...(await importOriginal<typeof import('esm-env')>()),
	BROWSER: true
}))
vi.mock('$lib/userScopedStorage', () => ({ scopedKey: (base: string) => `${base}::u` }))

const { recordHandEdit, removeHandEdit, sessionHandEdits } = await import(
	'./sessionHandEdits.svelte'
)

const STORED = 'windmill-session-hand-edits::u:s1'

beforeEach(() => localStorage.clear())

describe('session hand edits', () => {
	it('keeps the entries another tab stored since this one last read', () => {
		recordHandEdit('s1', 'script', 'u/a/one')
		// Another tab adds its own entry directly in the shared store.
		localStorage.setItem(STORED, JSON.stringify(['script:u/a/one', 'flow:u/a/two']))

		recordHandEdit('s1', 'raw_app', 'u/a/three')
		removeHandEdit('s1', 'script', 'u/a/one')

		expect([...sessionHandEdits('s1')].sort()).toEqual(['flow:u/a/two', 'raw_app:u/a/three'])
	})
})
