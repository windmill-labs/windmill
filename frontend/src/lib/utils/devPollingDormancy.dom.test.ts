import { afterEach, describe, expect, it, vi } from 'vitest'

// Freshly imported per case: install is idempotent by design, so a shared module instance
// would make the second case a no-op against a restored `window.setInterval`.
async function install() {
	vi.resetModules()
	const module = await import('./devPollingDormancy')
	module.installDevPollingDormancy()
}

// The patch is invisible in production and has no UI, so the freeze/re-arm pair is the
// only thing standing between a forgotten tab and a dev server that never gets reclaimed.
describe('devPollingDormancy', () => {
	const original = { setInterval: window.setInterval, clearInterval: window.clearInterval }

	afterEach(() => {
		window.setInterval = original.setInterval
		window.clearInterval = original.clearInterval
		// The install guard lives on `window`, so it is part of the global state to restore.
		delete (window as unknown as Record<string, boolean>).__wmDevPollingDormancyInstalled
		vi.useRealTimers()
		vi.unstubAllEnvs()
		vi.restoreAllMocks()
	})

	function setHidden(hidden: boolean) {
		Object.defineProperty(document, 'hidden', { value: hidden, configurable: true })
		document.dispatchEvent(new Event('visibilitychange'))
	}

	it('freezes intervals once the tab goes inactive and re-arms them on return', async () => {
		vi.useFakeTimers()
		vi.stubEnv('VITE_DEV_DORMANT_MS', '1000')
		vi.spyOn(document, 'hasFocus').mockReturnValue(true)
		setHidden(false)

		await install()

		const tick = vi.fn()
		const handle = window.setInterval(tick, 100)
		vi.advanceTimersByTime(300)
		expect(tick).toHaveBeenCalledTimes(3)

		setHidden(true)
		vi.advanceTimersByTime(1000)
		tick.mockClear()
		vi.advanceTimersByTime(500)
		expect(tick).not.toHaveBeenCalled()

		setHidden(false)
		vi.advanceTimersByTime(300)
		expect(tick).toHaveBeenCalledTimes(3)

		// Clearing through the patched handle must still stop the underlying timer.
		window.clearInterval(handle)
		tick.mockClear()
		vi.advanceTimersByTime(300)
		expect(tick).not.toHaveBeenCalled()
	})

	it('does not re-patch when the module itself is hot-replaced', async () => {
		vi.useFakeTimers()
		vi.spyOn(document, 'hasFocus').mockReturnValue(true)
		setHidden(false)

		await install()
		const patched = window.setInterval

		// A fresh module instance is what HMR hands the layout on the next edit.
		await install()

		expect(window.setInterval).toBe(patched)
	})

	it('arms intervals registered while dormant only once the tab is back', async () => {
		vi.useFakeTimers()
		vi.stubEnv('VITE_DEV_DORMANT_MS', '1000')
		vi.spyOn(document, 'hasFocus').mockReturnValue(true)
		setHidden(false)

		await install()

		setHidden(true)
		vi.advanceTimersByTime(1000)

		const tick = vi.fn()
		window.setInterval(tick, 100)
		vi.advanceTimersByTime(500)
		expect(tick).not.toHaveBeenCalled()

		setHidden(false)
		vi.advanceTimersByTime(200)
		expect(tick).toHaveBeenCalledTimes(2)
	})
})
