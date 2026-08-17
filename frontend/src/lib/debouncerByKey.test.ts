import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { createDebouncerByKey } from './debouncerByKey.svelte'

describe('createDebouncerByKey', () => {
	beforeEach(() => {
		vi.useFakeTimers()
	})
	afterEach(() => {
		vi.useRealTimers()
	})

	it('fires the task after debounceMs', () => {
		const d = createDebouncerByKey({ debounceMs: 1000, maxDebounceMs: 5000 })
		const fn = vi.fn()
		d.schedule('k', fn)
		expect(d.isPending('k')).toBe(true)
		vi.advanceTimersByTime(999)
		expect(fn).not.toHaveBeenCalled()
		vi.advanceTimersByTime(1)
		expect(fn).toHaveBeenCalledTimes(1)
		expect(d.isPending('k')).toBe(false)
	})

	it('keeps only the latest task and pushes the timer on each schedule', () => {
		const d = createDebouncerByKey({ debounceMs: 1000, maxDebounceMs: 10000 })
		const first = vi.fn()
		const second = vi.fn()
		d.schedule('k', first)
		vi.advanceTimersByTime(800)
		d.schedule('k', second) // resets the 1000ms window, replaces task
		vi.advanceTimersByTime(999)
		expect(second).not.toHaveBeenCalled()
		vi.advanceTimersByTime(1)
		expect(first).not.toHaveBeenCalled() // replaced — never ran
		expect(second).toHaveBeenCalledTimes(1)
	})

	it('caps the chain at maxDebounceMs from its start under a constant trickle', () => {
		const d = createDebouncerByKey({ debounceMs: 1000, maxDebounceMs: 2500 })
		const fn = vi.fn()
		// Re-schedule every 500ms — without the ceiling the 1000ms window
		// would never elapse, but maxDebounceMs forces a fire by t=2500.
		d.schedule('k', fn) // chainStart = 0
		for (let t = 500; t <= 2500; t += 500) {
			vi.advanceTimersByTime(500)
			if (t < 2500) d.schedule('k', fn)
		}
		expect(fn).toHaveBeenCalledTimes(1) // fired at the 2500ms ceiling
	})

	it('starts a fresh chain (and ceiling) after a fire', () => {
		const d = createDebouncerByKey({ debounceMs: 1000, maxDebounceMs: 2000 })
		const fn = vi.fn()
		d.schedule('k', fn)
		vi.advanceTimersByTime(1000)
		expect(fn).toHaveBeenCalledTimes(1)
		// Next schedule is a new chain — its own full debounce applies.
		d.schedule('k', fn)
		vi.advanceTimersByTime(999)
		expect(fn).toHaveBeenCalledTimes(1)
		vi.advanceTimersByTime(1)
		expect(fn).toHaveBeenCalledTimes(2)
	})

	it('cancel stops a pending task and reports whether there was one', () => {
		const d = createDebouncerByKey({ debounceMs: 1000, maxDebounceMs: 5000 })
		const fn = vi.fn()
		d.schedule('k', fn)
		expect(d.cancel('k')).toBe(true)
		expect(d.isPending('k')).toBe(false)
		vi.advanceTimersByTime(5000)
		expect(fn).not.toHaveBeenCalled()
		expect(d.cancel('k')).toBe(false) // nothing left to cancel
	})

	it('keeps keys independent', () => {
		const d = createDebouncerByKey({ debounceMs: 1000, maxDebounceMs: 5000 })
		const a = vi.fn()
		const b = vi.fn()
		d.schedule('a', a)
		d.schedule('b', b)
		d.cancel('a')
		vi.advanceTimersByTime(1000)
		expect(a).not.toHaveBeenCalled()
		expect(b).toHaveBeenCalledTimes(1)
	})
})
