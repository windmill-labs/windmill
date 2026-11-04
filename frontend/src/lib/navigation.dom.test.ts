import { describe, it, expect, vi, beforeEach } from 'vitest'

vi.mock('$app/navigation', () => ({ goto: vi.fn() }))

import { openOnNewTabClick } from './navigation'

describe('openOnNewTabClick', () => {
	let row: HTMLDivElement
	let link: HTMLAnchorElement
	let open: ReturnType<typeof vi.fn>

	beforeEach(() => {
		document.body.innerHTML =
			'<div id="row"><span id="label">x</span><a id="link" href="#run">r</a></div>'
		row = document.getElementById('row') as HTMLDivElement
		link = document.getElementById('link') as HTMLAnchorElement
		open = vi.fn()
		window.open = open as unknown as typeof window.open
	})

	function fire(target: Element, init: MouseEventInit, opts?: { modifiers?: boolean }) {
		let opened = false
		row.addEventListener('click', (e) => (opened = openOnNewTabClick(e, '/item', opts)), {
			once: true
		})
		target.dispatchEvent(new MouseEvent('click', { bubbles: true, ...init }))
		return opened
	}

	it('opens the row href on middle and ctrl/cmd clicks, not on a plain click', () => {
		const label = document.getElementById('label')!
		expect(fire(label, {})).toBe(false)
		expect(fire(label, { button: 1 })).toBe(true)
		expect(fire(label, { ctrlKey: true })).toBe(true)
		expect(fire(label, { metaKey: true })).toBe(true)
		expect(open).toHaveBeenCalledTimes(3)
		expect(open).toHaveBeenCalledWith('/item', '_blank')
	})

	it('leaves a click on a nested link to that link', () => {
		expect(fire(link, { button: 1 })).toBe(false)
		expect(open).not.toHaveBeenCalled()
	})

	it('ignores ctrl/cmd-click when modifiers is false', () => {
		const label = document.getElementById('label')!
		expect(fire(label, { ctrlKey: true }, { modifiers: false })).toBe(false)
		expect(fire(label, { button: 1 }, { modifiers: false })).toBe(true)
	})
})
