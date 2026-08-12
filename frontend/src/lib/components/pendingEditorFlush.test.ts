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

	it('flushes registered editors, and stops once they unmount', () => {
		let flushed = 0
		const deregister = registerPendingEditor({ flushPendingChanges: () => flushed++ })
		flushAllPendingEditorChanges()
		deregister()
		flushAllPendingEditorChanges()
		expect(flushed).toBe(1)
	})
})
