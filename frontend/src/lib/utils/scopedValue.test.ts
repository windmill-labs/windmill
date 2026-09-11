import { describe, expect, it } from 'vitest'
import { scopedValue, tagged } from './scopedValue'

// The ordering these assert is the one `resource` does not provide, and the one whose
// absence produced the stale-workspace and A→B→A defects this guard replaced.
describe('scopedValue', () => {
	it('holds a value only for the key it describes', () => {
		const held = scopedValue<number>()
		expect(held('a', undefined)).toBe(undefined)
		expect(held('a', { key: 'a', seq: 1, value: 1 })).toBe(1)
		// Switched to b, nothing fetched for it yet: a's value must not stand in.
		expect(held('b', { key: 'a', seq: 1, value: 1 })).toBe(undefined)
		expect(held('b', { key: 'b', seq: 2, value: 2 })).toBe(2)
	})

	it('ignores an answer for a scope we left, instead of publishing or erasing', () => {
		const held = scopedValue<number>()
		held('a', { key: 'a', seq: 1, value: 1 })
		held('b', { key: 'b', seq: 2, value: 2 })
		// A's slow response lands after B resolved: neither replaces B's value nor blanks it.
		expect(held('b', { key: 'a', seq: 1, value: 99 })).toBe(2)
	})

	it('ignores an answer overtaken by a newer one for the same key', () => {
		const held = scopedValue<number>()
		// Two fetches for one key — a refetch landing on an in-flight load, or a second
		// invalidation — resolving inverted. The later-issued value must win.
		expect(held('a', { key: 'a', seq: 2, value: 20 })).toBe(20)
		expect(held('a', { key: 'a', seq: 1, value: 10 })).toBe(20)
	})

	it('keeps the value across a re-read of the same key', () => {
		const held = scopedValue<number>()
		held('a', { key: 'a', seq: 1, value: 1 })
		// A refetch leaves the previous value in place until the new one lands, so the
		// display never blanks mid-refresh.
		expect(held('a', { key: 'a', seq: 1, value: 1 })).toBe(1)
		expect(held('a', { key: 'a', seq: 2, value: 5 })).toBe(5)
	})

	it('treats returning to a key as unknown until it is fetched again', () => {
		const held = scopedValue<number>()
		held('a', { key: 'a', seq: 1, value: 1 })
		held('b', { key: 'b', seq: 2, value: 2 })
		expect(held('a', undefined)).toBe(undefined)
	})

	it('orders a late answer against the read issued on returning to its key', () => {
		const held = scopedValue<number>()
		// A's first read is still in flight when we leave for B, so nothing for A is held.
		expect(held('b', { key: 'b', seq: 2, value: 2 })).toBe(2)
		// Back on A, that late answer is the only value describing A, so it stands...
		expect(held('a', { key: 'a', seq: 1, value: 10 })).toBe(10)
		// ...until the read issued on returning lands, and cannot come back afterwards.
		expect(held('a', { key: 'a', seq: 3, value: 30 })).toBe(30)
		expect(held('a', { key: 'a', seq: 1, value: 10 })).toBe(30)
	})

	it('stamps issue order even when responses resolve inverted', async () => {
		const settle: Array<(v: number) => void> = []
		const fetch = tagged((_key: string) => new Promise<number>((r) => settle.push(r)))
		const first = fetch('a')
		const second = fetch('a')
		// Resolve the second request first, then the first: the seq must reflect the
		// order they were *issued*, not the order they came back.
		settle[1](20)
		settle[0](10)
		expect(await first).toEqual({ key: 'a', seq: 1, value: 10 })
		expect(await second).toEqual({ key: 'a', seq: 2, value: 20 })

		const held = scopedValue<number>()
		expect(held('a', await second)).toBe(20)
		expect(held('a', await first)).toBe(20)
	})
})
