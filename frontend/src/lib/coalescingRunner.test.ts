import { describe, it, expect, vi } from 'vitest'
import { createCoalescingKeyedRunner, CoalescingDisplacedError } from './coalescingRunner.svelte'

/** A promise plus its resolve/reject, so a test can decide exactly when a
 * task settles. */
function deferred<T = void>() {
	let resolve!: (v: T) => void
	let reject!: (e: unknown) => void
	const promise = new Promise<T>((res, rej) => {
		resolve = res
		reject = rej
	})
	return { promise, resolve, reject }
}

describe('createCoalescingKeyedRunner', () => {
	it('runs a submitted task immediately when the key is idle', () => {
		const runner = createCoalescingKeyedRunner()
		const fn = vi.fn(() => Promise.resolve())
		runner.submit('k', fn)
		expect(fn).toHaveBeenCalledTimes(1)
		expect(runner.isRunning('k')).toBe(true)
	})

	it('clears isRunning once the in-flight task settles', async () => {
		const runner = createCoalescingKeyedRunner()
		const d = deferred()
		runner.submit('k', () => d.promise)
		expect(runner.isRunning('k')).toBe(true)
		d.resolve()
		await d.promise
		await Promise.resolve()
		expect(runner.isRunning('k')).toBe(false)
	})

	it('coalesces a burst down to in-flight + latest-pending', async () => {
		const runner = createCoalescingKeyedRunner()
		const d = deferred()
		const f = vi.fn(() => d.promise)
		const g = vi.fn(() => Promise.resolve())
		const h = vi.fn(() => Promise.resolve())

		runner.submit('k', f) // runs now
		runner.submit('k', g) // pending
		runner.submit('k', h) // displaces g; h is the only pending

		expect(f).toHaveBeenCalledTimes(1)
		expect(g).not.toHaveBeenCalled()
		expect(h).not.toHaveBeenCalled()

		d.resolve()
		await d.promise
		await Promise.resolve()
		await Promise.resolve()

		expect(g).not.toHaveBeenCalled() // displaced — never ran
		expect(h).toHaveBeenCalledTimes(1)
		expect(runner.isRunning('k')).toBe(false)
	})

	it('keeps different keys independent', () => {
		const runner = createCoalescingKeyedRunner()
		const a = vi.fn(() => new Promise<void>(() => {}))
		const b = vi.fn(() => new Promise<void>(() => {}))
		runner.submit('a', a)
		runner.submit('b', b) // different key → runs immediately too
		expect(a).toHaveBeenCalledTimes(1)
		expect(b).toHaveBeenCalledTimes(1)
		expect(runner.isRunning('a')).toBe(true)
		expect(runner.isRunning('b')).toBe(true)
	})

	it('submitAndWait resolves with the task result', async () => {
		const runner = createCoalescingKeyedRunner()
		await expect(runner.submitAndWait('k', () => Promise.resolve(42))).resolves.toBe(42)
	})

	it('submitAndWait rejects with the task error', async () => {
		const runner = createCoalescingKeyedRunner()
		await expect(
			runner.submitAndWait('k', () => Promise.reject(new Error('boom')))
		).rejects.toThrow('boom')
	})

	it('rejects a displaced submitAndWait with CoalescingDisplacedError', async () => {
		const runner = createCoalescingKeyedRunner()
		const d = deferred()
		runner.submit('k', () => d.promise) // hold the key busy
		const dropped = runner.submitAndWait('k', () => Promise.resolve('a')) // pending
		runner.submit('k', () => Promise.resolve('b')) // displaces it
		await expect(dropped).rejects.toBeInstanceOf(CoalescingDisplacedError)
		d.resolve()
	})

	it('cancel drops the pending task and rejects its awaiter', async () => {
		const runner = createCoalescingKeyedRunner()
		const d = deferred()
		const pending = vi.fn(() => Promise.resolve())
		runner.submit('k', () => d.promise) // in flight
		const awaited = runner.submitAndWait('k', pending) // pending

		expect(runner.cancel('k')).toBe(true)
		await expect(awaited).rejects.toBeInstanceOf(CoalescingDisplacedError)

		d.resolve()
		await d.promise
		await Promise.resolve()
		expect(pending).not.toHaveBeenCalled() // cancelled before it could run
	})

	it('cancel returns false when there is no pending task', () => {
		const runner = createCoalescingKeyedRunner()
		expect(runner.cancel('k')).toBe(false)
		runner.submit('k', () => new Promise<void>(() => {})) // running, nothing pending
		expect(runner.cancel('k')).toBe(false)
	})

	it('does not abort the in-flight task on cancel', async () => {
		const runner = createCoalescingKeyedRunner()
		const d = deferred()
		const inflight = vi.fn(() => d.promise)
		runner.submit('k', inflight)
		runner.cancel('k') // only affects pending; nothing pending here
		expect(runner.isRunning('k')).toBe(true)
		d.resolve()
		await d.promise
		await Promise.resolve()
		expect(inflight).toHaveBeenCalledTimes(1)
	})
})
