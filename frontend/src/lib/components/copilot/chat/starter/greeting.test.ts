import { beforeEach, describe, expect, it, vi } from 'vitest'

const scoped = vi.hoisted(() => ({ key: 'welcomed::ada@example.com' as string | undefined }))
vi.mock('$lib/userScopedStorage', () => ({ scopedKey: () => scoped.key }))

import { takeFirstVisit } from './greeting'

const DAY = 24 * 60 * 60 * 1000
const daysAgo = (n: number) => new Date(Date.now() - n * DAY).toISOString()

const store = new Map<string, string>()
vi.stubGlobal('localStorage', {
	getItem: (k: string) => store.get(k) ?? null,
	setItem: (k: string, v: string) => void store.set(k, v)
})

beforeEach(() => {
	store.clear()
	scoped.key = 'welcomed::ada@example.com'
})

describe('takeFirstVisit', () => {
	it('welcomes a new member once per workspace, for the whole page load', () => {
		expect(takeFirstVisit('acme', daysAgo(2))).toBe(true)
		// Back on Home or the empty session in the same load: still the welcome.
		expect(takeFirstVisit('acme', daysAgo(2))).toBe(true)
		expect(JSON.parse(store.get('welcomed::ada@example.com')!)).toEqual(['acme'])
	})

	it('greets by time of day for an older member, or a workspace already welcomed', () => {
		expect(takeFirstVisit('old-team', daysAgo(45))).toBe(false)
		store.set('welcomed::ada@example.com', JSON.stringify(['seen-before']))
		expect(takeFirstVisit('seen-before', daysAgo(1))).toBe(false)
	})

	it('records nothing before the user is known', () => {
		scoped.key = undefined
		expect(takeFirstVisit('fresh', daysAgo(1))).toBe(false)
		expect(store.size).toBe(0)
	})
})
