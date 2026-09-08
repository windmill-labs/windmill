import { describe, it, expect, beforeEach } from 'vitest'
import type { UserWorkspace } from './stores'
import { recordForkParent, getRememberedForkParent, forgetForkParent } from './forkParentMemory'

const MAX_ENTRIES = 50

function ws(id: string, parent_workspace_id?: string): UserWorkspace {
	return {
		id,
		name: id,
		username: 'admin',
		color: undefined,
		operator_settings: undefined,
		disabled: false,
		parent_workspace_id
	}
}

beforeEach(() => {
	localStorage.clear()
})

describe('recordForkParent / getRememberedForkParent', () => {
	it('remembers the parent of a fork present in the list', () => {
		recordForkParent('wm-fork-a', [ws('wm-fork-a', 'parent'), ws('parent')])
		expect(getRememberedForkParent('wm-fork-a')).toBe('parent')
	})

	it('is a no-op when the workspace is absent from the list', () => {
		recordForkParent('wm-fork-a', [ws('parent')])
		expect(getRememberedForkParent('wm-fork-a')).toBeUndefined()
	})

	it('is a no-op when the workspace has no parent', () => {
		recordForkParent('parent', [ws('parent')])
		expect(getRememberedForkParent('parent')).toBeUndefined()
	})

	it('is a no-op for an undefined workspace id', () => {
		recordForkParent(undefined, [ws('wm-fork-a', 'parent')])
		expect(getRememberedForkParent('wm-fork-a')).toBeUndefined()
	})

	it('never clobbers a prior mapping once the fork disappears from the list', () => {
		// Recorded while reachable...
		recordForkParent('wm-fork-a', [ws('wm-fork-a', 'parent'), ws('parent')])
		// ...then the fork is gone from a later list — the mapping must survive so
		// recovery can still find the parent.
		recordForkParent('wm-fork-a', [ws('parent')])
		expect(getRememberedForkParent('wm-fork-a')).toBe('parent')
	})

	it('handles workspace ids that collide with Object prototype members', () => {
		for (const id of ['__proto__', 'constructor', 'toString']) {
			recordForkParent(id, [ws(id, `parent-${id}`), ws(`parent-${id}`)])
			expect(getRememberedForkParent(id)).toBe(`parent-${id}`)
			forgetForkParent(id)
			expect(getRememberedForkParent(id)).toBeUndefined()
		}
	})
})

describe('forgetForkParent', () => {
	it('removes a remembered mapping', () => {
		recordForkParent('wm-fork-a', [ws('wm-fork-a', 'parent')])
		forgetForkParent('wm-fork-a')
		expect(getRememberedForkParent('wm-fork-a')).toBeUndefined()
	})

	it('is a no-op for an unknown fork', () => {
		expect(() => forgetForkParent('wm-fork-missing')).not.toThrow()
	})
})

describe('bounded storage', () => {
	it('evicts the oldest entry once past MAX_ENTRIES', () => {
		for (let i = 0; i < MAX_ENTRIES; i++) {
			recordForkParent(`wm-fork-${i}`, [ws(`wm-fork-${i}`, `parent-${i}`)])
		}
		// One more tips it over the cap and trims the oldest (index 0).
		recordForkParent(`wm-fork-${MAX_ENTRIES}`, [
			ws(`wm-fork-${MAX_ENTRIES}`, `parent-${MAX_ENTRIES}`)
		])
		expect(getRememberedForkParent('wm-fork-0')).toBeUndefined()
		expect(getRememberedForkParent(`wm-fork-${MAX_ENTRIES}`)).toBe(`parent-${MAX_ENTRIES}`)
		expect(getRememberedForkParent('wm-fork-1')).toBe('parent-1')
	})

	it('updating a fork to a new parent moves it to the end so it survives the next eviction', () => {
		for (let i = 0; i < MAX_ENTRIES; i++) {
			recordForkParent(`wm-fork-${i}`, [ws(`wm-fork-${i}`, `parent-${i}`)])
		}
		// A changed value re-inserts the key at the end of the insertion order.
		recordForkParent('wm-fork-0', [ws('wm-fork-0', 'reparented')])
		// Push past the cap: the now-oldest (index 1) is evicted, not index 0.
		recordForkParent(`wm-fork-${MAX_ENTRIES}`, [
			ws(`wm-fork-${MAX_ENTRIES}`, `parent-${MAX_ENTRIES}`)
		])
		expect(getRememberedForkParent('wm-fork-0')).toBe('reparented')
		expect(getRememberedForkParent('wm-fork-1')).toBeUndefined()
	})
})

describe('corrupted storage', () => {
	it('tolerates non-JSON content', () => {
		localStorage.setItem('fork_parents', 'not json{')
		expect(getRememberedForkParent('wm-fork-a')).toBeUndefined()
		// A subsequent write recovers cleanly.
		recordForkParent('wm-fork-a', [ws('wm-fork-a', 'parent')])
		expect(getRememberedForkParent('wm-fork-a')).toBe('parent')
	})

	it('tolerates a non-object JSON value', () => {
		localStorage.setItem('fork_parents', '42')
		expect(getRememberedForkParent('wm-fork-a')).toBeUndefined()
	})
})
