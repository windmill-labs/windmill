/**
 * Insert clipboard text into the focused form control.
 * Used when the VS Code preview hosts /dev in a nested iframe: Cmd+V never
 * reaches the frame, so the extension posts { type: 'clipboardPaste', text }.
 */
export function insertClipboardText(text: string, doc: Document = document): boolean {
	const el = doc.activeElement
	if (el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement) {
		const start = el.selectionStart ?? el.value.length
		const end = el.selectionEnd ?? el.value.length
		const next = el.value.slice(0, start) + text + el.value.slice(end)
		const proto = Object.getOwnPropertyDescriptor(
			el instanceof HTMLTextAreaElement
				? HTMLTextAreaElement.prototype
				: HTMLInputElement.prototype,
			'value'
		)
		proto?.set?.call(el, next)
		el.setSelectionRange(start + text.length, start + text.length)
		el.dispatchEvent(new Event('input', { bubbles: true }))
		return true
	}
	if (el instanceof HTMLElement && el.isContentEditable) {
		return doc.execCommand('insertText', false, text)
	}
	return false
}
