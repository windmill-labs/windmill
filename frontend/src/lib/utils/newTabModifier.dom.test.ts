import { afterEach, describe, expect, it, vi } from 'vitest'
import { newTabModifier } from './newTabModifier.svelte'

const onPlatform = (userAgent: string) => vi.stubGlobal('navigator', { userAgent })
const LINUX = 'Mozilla/5.0 (X11; Linux x86_64)'
const MAC = 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)'

const key = (type: 'keydown' | 'keyup', init: KeyboardEventInit) =>
	window.dispatchEvent(new KeyboardEvent(type, init))

describe('newTabModifier', () => {
	afterEach(() => {
		window.dispatchEvent(new Event('blur'))
		vi.unstubAllGlobals()
	})

	it('reads ctrl off macOS', () => {
		onPlatform(LINUX)
		key('keydown', { key: 'Control', ctrlKey: true })
		expect(newTabModifier.held).toBe(true)
		key('keyup', { key: 'Control', ctrlKey: false })
		expect(newTabModifier.held).toBe(false)
	})

	it('reads meta on macOS, never ctrl', () => {
		onPlatform(MAC)
		key('keydown', { key: 'Meta', metaKey: true })
		expect(newTabModifier.held).toBe(true)
		// macOS ctrl+click is a secondary click, so it must not read as a new-tab modifier.
		key('keydown', { key: 'Control', ctrlKey: true, metaKey: false })
		expect(newTabModifier.held).toBe(false)
	})

	// The flag follows the latest event, so a keyup this window never received cannot leave it
	// stuck on.
	it('recovers from a keyup it never saw', () => {
		onPlatform(LINUX)
		key('keydown', { key: 'Control', ctrlKey: true })
		expect(newTabModifier.held).toBe(true)

		// The keyup went to another window; the next event this one sees is unmodified.
		key('keydown', { key: 'a', ctrlKey: false })
		expect(newTabModifier.held).toBe(false)
	})

	// Editors and menus stop keydown propagation to keep their own shortcuts, so a bubble-phase
	// listener would go blind whenever focus sits in one.
	it('sees a keydown that a focused element stops from propagating', () => {
		onPlatform(LINUX)
		const input = document.createElement('input')
		input.addEventListener('keydown', (e) => e.stopPropagation())
		document.body.append(input)

		input.dispatchEvent(
			new KeyboardEvent('keydown', { key: 'Control', ctrlKey: true, bubbles: true })
		)
		expect(newTabModifier.held).toBe(true)

		input.remove()
	})

	it('clears on blur, since the keyup lands elsewhere', () => {
		onPlatform(LINUX)
		key('keydown', { key: 'Control', ctrlKey: true })
		expect(newTabModifier.held).toBe(true)

		window.dispatchEvent(new Event('blur'))
		expect(newTabModifier.held).toBe(false)
	})
})
