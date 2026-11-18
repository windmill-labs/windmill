import { describe, it, expect } from 'vitest'
import {
	anyEditorUnparseable,
	setEditorUnparseable,
	registerPendingEditor,
	flushAllPendingEditorChanges
} from './pendingEditorFlush'

describe('pendingEditorFlush', () => {
	it('reports unparseable text until the editor clears it', () => {
		const editor = {}
		expect(anyEditorUnparseable()).toBe(false)
		setEditorUnparseable(editor, true)
		expect(anyEditorUnparseable()).toBe(true)
		setEditorUnparseable(editor, false)
		expect(anyEditorUnparseable()).toBe(false)
	})

	it("scopes the check to one form, so another form's broken editor does not block it", () => {
		const insideA = {} as Element
		const formA = { contains: (el: Element) => el === insideA } as Element
		const formB = { contains: () => false } as unknown as Element
		const editor = {}
		setEditorUnparseable(editor, true, insideA)
		expect(anyEditorUnparseable(formA)).toBe(true)
		expect(anyEditorUnparseable(formB)).toBe(false)
		expect(anyEditorUnparseable()).toBe(true)
		setEditorUnparseable(editor, false)
	})

	it('flushes registered editors, and stops once they unmount', () => {
		let flushed = 0
		const deregister = registerPendingEditor({ flushPendingChanges: () => flushed++ })
		flushAllPendingEditorChanges()
		deregister()
		flushAllPendingEditorChanges()
		expect(flushed).toBe(1)
	})
})
