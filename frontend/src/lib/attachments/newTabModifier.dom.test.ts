import { afterEach, describe, expect, it, vi } from 'vitest'
import { newTabModifier, trackNewTabModifier } from './newTabModifier.svelte'

const onPlatform = (userAgent: string) => vi.stubGlobal('navigator', { userAgent })
const LINUX = 'Mozilla/5.0 (X11; Linux x86_64)'
const MAC = 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)'

const attached: (() => void)[] = []

/** Attach to a fresh element and return it with its cleanup, as `{@attach}` would. */
function pill() {
	const node = document.createElement('span')
	document.body.append(node)
	const cleanup = trackNewTabModifier(node) as () => void
	attached.push(cleanup)
	const hover = (init: MouseEventInit = {}) =>
		node.dispatchEvent(new MouseEvent('mouseenter', init))
	const unhover = () => node.dispatchEvent(new MouseEvent('mouseleave'))
	return { hover, unhover, cleanup }
}

const keydown = (init: KeyboardEventInit) =>
	window.dispatchEvent(new KeyboardEvent('keydown', init))

describe('trackNewTabModifier', () => {
	// The module state and its window listeners outlive the DOM, so every case has to be torn
	// down through the attachment rather than by emptying the body.
	afterEach(() => {
		attached.splice(0).forEach((cleanup) => cleanup())
		document.body.replaceChildren()
		vi.unstubAllGlobals()
	})

	// The hover event carries the live modifier state, so a modifier pressed before the pointer
	// arrived (or while this window was unfocused) is picked up rather than read as false.
	it('seeds from the hover event, per platform', () => {
		onPlatform(LINUX)
		const linux = pill()
		linux.hover({ ctrlKey: true })
		expect(newTabModifier.held).toBe(true)
		linux.unhover()

		onPlatform(MAC)
		const mac = pill()
		// macOS ctrl+click is a secondary click, so it must not read as a new-tab modifier.
		mac.hover({ ctrlKey: true })
		expect(newTabModifier.held).toBe(false)
		mac.hover({ metaKey: true })
		expect(newTabModifier.held).toBe(true)
	})

	// Editors and menus stop keydown propagation to keep their own shortcuts, so a bubble-phase
	// listener would go blind whenever focus sits in one.
	it('sees a keydown that a focused element stops from propagating', () => {
		onPlatform(LINUX)
		const { hover } = pill()
		hover()
		const input = document.createElement('input')
		input.addEventListener('keydown', (e) => e.stopPropagation())
		document.body.append(input)

		input.dispatchEvent(
			new KeyboardEvent('keydown', { key: 'Control', ctrlKey: true, bubbles: true })
		)
		expect(newTabModifier.held).toBe(true)
	})

	it('stops tracking once unhovered', () => {
		onPlatform(LINUX)
		const { hover, unhover } = pill()
		hover({ ctrlKey: true })
		unhover()
		expect(newTabModifier.held).toBe(false)

		keydown({ key: 'Control', ctrlKey: true })
		expect(newTabModifier.held).toBe(false)
	})

	it('stops tracking when the element is destroyed while hovered', () => {
		onPlatform(LINUX)
		const { hover, cleanup } = pill()
		hover({ ctrlKey: true })
		cleanup()
		expect(newTabModifier.held).toBe(false)

		keydown({ key: 'Control', ctrlKey: true })
		expect(newTabModifier.held).toBe(false)
	})

	// A pill destroyed elsewhere in the transcript must not tear down the hovered pill's listeners.
	it('keeps tracking when a different element is destroyed', () => {
		onPlatform(LINUX)
		const { hover } = pill()
		const other = pill()
		hover()
		other.cleanup()

		keydown({ key: 'Control', ctrlKey: true })
		expect(newTabModifier.held).toBe(true)
	})
})
