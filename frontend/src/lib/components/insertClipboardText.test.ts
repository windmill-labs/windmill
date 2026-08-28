import { describe, expect, it } from 'vitest'

import { insertClipboardText } from './insertClipboardText'

describe('insertClipboardText', () => {
	it('replaces the selection in a focused input', () => {
		const input = document.createElement('input')
		document.body.appendChild(input)
		input.value = 'abxxef'
		input.focus()
		input.setSelectionRange(2, 4)
		expect(insertClipboardText('cd')).toBe(true)
		expect(input.value).toBe('abcdef')
		input.remove()
	})

	it('appends into a textarea with no selection', () => {
		const area = document.createElement('textarea')
		document.body.appendChild(area)
		area.value = 'id-'
		area.focus()
		area.setSelectionRange(3, 3)
		expect(insertClipboardText('99')).toBe(true)
		expect(area.value).toBe('id-99')
		area.remove()
	})

	it('returns false when nothing is focused', () => {
		;(document.activeElement as HTMLElement | null)?.blur?.()
		expect(insertClipboardText('nope')).toBe(false)
	})
})
