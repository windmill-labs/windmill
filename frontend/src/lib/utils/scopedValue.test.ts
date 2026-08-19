import { describe, expect, it } from 'vitest'
import { scopedValue } from './scopedValue'

// The ordering these assert is the one `resource` does not provide, and the one whose
// absence produced the stale-workspace and A→B→A defects this guard replaced.
describe('scopedValue', () => {
	it('holds a value only for the key it describes', () => {
		const held = scopedValue<number>()
		expect(held('a', undefined)).toBe(undefined)
		expect(held('a', { key: 'a', value: 1 })).toBe(1)
		// Switched to b, nothing fetched for it yet: a's value must not stand in.
		expect(held('b', { key: 'a', value: 1 })).toBe(undefined)
		expect(held('b', { key: 'b', value: 2 })).toBe(2)
	})

	it('ignores a superseded answer instead of publishing or erasing', () => {
		const held = scopedValue<number>()
		held('a', { key: 'a', value: 1 })
		held('b', { key: 'b', value: 2 })
		// A's slow response lands after B resolved: neither replaces B's value nor blanks it.
		expect(held('b', { key: 'a', value: 99 })).toBe(2)
	})

	it('keeps the value across a re-read of the same key', () => {
		const held = scopedValue<number>()
		held('a', { key: 'a', value: 1 })
		// A refetch leaves the previous value in place until the new one lands, so the
		// display never blanks mid-refresh.
		expect(held('a', { key: 'a', value: 1 })).toBe(1)
		expect(held('a', { key: 'a', value: 5 })).toBe(5)
	})

	it('treats returning to a key as unknown until it is fetched again', () => {
		const held = scopedValue<number>()
		held('a', { key: 'a', value: 1 })
		held('b', { key: 'b', value: 2 })
		expect(held('a', undefined)).toBe(undefined)
	})
})
