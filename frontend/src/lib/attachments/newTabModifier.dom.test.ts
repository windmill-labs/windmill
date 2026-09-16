import { afterEach, describe, expect, it, vi } from 'vitest'
import { newTabModifier } from './newTabModifier.svelte'

const onPlatform = (userAgent: string) => vi.stubGlobal('navigator', { userAgent })
const LINUX = 'Mozilla/5.0 (X11; Linux x86_64)'
const MAC = 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)'

const attached: (() => void)[] = []

/** Attach to a fresh element and return it with its cleanup, as `{@attach}` would. */
function pill() {
	const node = document.createElement('span')
	document.body.append(node)
	const modifier = newTabModifier()
	const cleanup = modifier.attach(node) as () => void
	attached.push(cleanup)
	const hover = (init: MouseEventInit = {}) =>
		node.dispatchEvent(new MouseEvent('mouseenter', init))
	const move = (init: MouseEventInit = {}) => node.dispatchEvent(new MouseEvent('mousemove', init))
	const unhover = () => node.dispatchEvent(new MouseEvent('mouseleave'))
	return { modifier, hover, move, unhover, cleanup }
}

const keydown = (init: KeyboardEventInit) =>
	window.dispatchEvent(new KeyboardEvent('keydown', init))

describe('newTabModifier', () => {
	// The window listeners outlive the DOM, so every case has to be torn down through the
	// attachment rather than by emptying the body.
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
		expect(linux.modifier.held).toBe(true)

		onPlatform(MAC)
		const mac = pill()
		// macOS ctrl+click is a secondary click, so it must not read as a new-tab modifier.
		mac.hover({ ctrlKey: true })
		expect(mac.modifier.held).toBe(false)
		mac.hover({ metaKey: true })
		expect(mac.modifier.held).toBe(true)
	})

	// Editors and menus stop keydown propagation to keep their own shortcuts, so a bubble-phase
	// listener would go blind whenever focus sits in one.
	it('sees a keydown that a focused element stops from propagating', () => {
		onPlatform(LINUX)
		const { modifier, hover } = pill()
		hover()
		const input = document.createElement('input')
		input.addEventListener('keydown', (e) => e.stopPropagation())
		document.body.append(input)

		input.dispatchEvent(
			new KeyboardEvent('keydown', { key: 'Control', ctrlKey: true, bubbles: true })
		)
		expect(modifier.held).toBe(true)
	})

	// A modifier held across a keyboard app switch is cleared by the blur and delivers no keydown
	// on the way back, while the pointer parked on the pill fires no fresh mouseenter either.
	it('re-seeds from pointer movement after the window lost focus', () => {
		onPlatform(LINUX)
		const { modifier, hover, move } = pill()
		hover({ ctrlKey: true })
		window.dispatchEvent(new Event('blur'))
		expect(modifier.held).toBe(false)

		move({ ctrlKey: true })
		expect(modifier.held).toBe(true)
	})

	it('stops tracking once unhovered', () => {
		onPlatform(LINUX)
		const { modifier, hover, unhover } = pill()
		hover({ ctrlKey: true })
		unhover()
		expect(modifier.held).toBe(false)

		keydown({ key: 'Control', ctrlKey: true })
		expect(modifier.held).toBe(false)
	})

	it('stops tracking when the element is destroyed while hovered', () => {
		onPlatform(LINUX)
		const { modifier, hover, cleanup } = pill()
		hover({ ctrlKey: true })
		// Hovering again without leaving must not strand the first hover's listeners, which nothing
		// would then hold a reference to.
		hover({ ctrlKey: true })
		cleanup()
		expect(modifier.held).toBe(false)

		keydown({ key: 'Control', ctrlKey: true })
		expect(modifier.held).toBe(false)
	})
})
