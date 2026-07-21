import { languages } from 'monaco-editor/esm/vs/editor/editor.api'

import { editor as meditor } from 'monaco-editor/esm/vs/editor/editor.api'

export function editorConfig(
	code: string,
	lang: string,
	automaticLayout: boolean,
	fixedOverflowWidgets: boolean,
	relativeLineNumbers?: boolean
) {
	return {
		value: code,
		language: lang,
		automaticLayout,
		readOnly: false,
		fixedOverflowWidgets,
		lineDecorationsWidth: 10,
		lineNumbersMinChars: 3,
		lineNumbers: (relativeLineNumbers ?? false) ? ('relative' as const) : ('on' as const),
		scrollbar: { alwaysConsumeMouseWheel: false },
		folding: false,
		scrollBeyondLastLine: false,
		glyphMargin: false,
		minimap: {
			enabled: false
		},
		lightbulb: {
			enabled: meditor.ShowLightbulbIconMode.On
		},
		suggest: {
			showKeywords: true
		},
		bracketPairColorization: {
			enabled: true
		},
		'workbench.colorTheme': 'Default Dark Modern',
		workbench: {
			colorTheme: 'Default Dark Modern'
		},
		'bracketPairColorization.enabled': true,
		matchBrackets: 'always' as 'always'
	}
}

export const updateOptions = { tabSize: 2, insertSpaces: true }

let hoveredSwipeGuards = 0

function updateRootOverscroll() {
	const value = hoveredSwipeGuards > 0 ? 'none' : ''
	document.documentElement.style.overscrollBehaviorX = value
	document.body.style.overscrollBehaviorX = value
}

// On macOS Chromium, horizontal wheel overscroll triggers history back/forward
// navigation, so a sideways swipe over an editor navigates away. Cancelling
// wheel events is not reliable: only the first event of a trackpad gesture
// stream is cancelable, and Monaco stops propagation of events it consumes.
// Chromium instead consults `overscroll-behavior-x` on the ROOT element for
// this gesture (the editor container is not a native scroller, so setting it
// there has no effect) — toggle it while the pointer is over an editor.
export function preventHorizontalNavigationSwipe(el: HTMLElement) {
	let over = false
	const enter = () => {
		if (!over) {
			over = true
			hoveredSwipeGuards++
			updateRootOverscroll()
		}
	}
	const leave = () => {
		if (over) {
			over = false
			hoveredSwipeGuards--
			updateRootOverscroll()
		}
	}
	el.addEventListener('pointerenter', enter)
	el.addEventListener('pointerleave', leave)
	// A wheel event implies the pointer is over the editor even when
	// pointerenter never fired (editor mounted underneath a stationary cursor).
	el.addEventListener('wheel', enter, { passive: true })
	return {
		destroy: () => {
			el.removeEventListener('pointerenter', enter)
			el.removeEventListener('pointerleave', leave)
			el.removeEventListener('wheel', enter)
			// The action can be destroyed while hovered (editor inside a closing
			// drawer) — pointerleave never fires then, so release here.
			leave()
		}
	}
}

export function convertKind(kind: string): any {
	switch (kind) {
		case Kind.primitiveType:
		case Kind.keyword:
			return languages.CompletionItemKind.Keyword
		case Kind.variable:
		case Kind.localVariable:
			return languages.CompletionItemKind.Variable
		case Kind.memberVariable:
		case Kind.memberGetAccessor:
		case Kind.memberSetAccessor:
			return languages.CompletionItemKind.Field
		case Kind.function:
		case Kind.memberFunction:
		case Kind.constructSignature:
		case Kind.callSignature:
		case Kind.indexSignature:
			return languages.CompletionItemKind.Function
		case Kind.enum:
			return languages.CompletionItemKind.Enum
		case Kind.module:
			return languages.CompletionItemKind.Module
		case Kind.class:
			return languages.CompletionItemKind.Class
		case Kind.interface:
			return languages.CompletionItemKind.Interface
		case Kind.warning:
			return languages.CompletionItemKind.File
	}

	return languages.CompletionItemKind.Property
}

class Kind {
	public static unknown: string = ''
	public static keyword: string = 'keyword'
	public static script: string = 'script'
	public static module: string = 'module'
	public static class: string = 'class'
	public static interface: string = 'interface'
	public static type: string = 'type'
	public static enum: string = 'enum'
	public static variable: string = 'var'
	public static localVariable: string = 'local var'
	public static function: string = 'function'
	public static localFunction: string = 'local function'
	public static memberFunction: string = 'method'
	public static memberGetAccessor: string = 'getter'
	public static memberSetAccessor: string = 'setter'
	public static memberVariable: string = 'property'
	public static constructorImplementation: string = 'constructor'
	public static callSignature: string = 'call'
	public static indexSignature: string = 'index'
	public static constructSignature: string = 'construct'
	public static parameter: string = 'parameter'
	public static typeParameter: string = 'type parameter'
	public static primitiveType: string = 'primitive type'
	public static label: string = 'label'
	public static alias: string = 'alias'
	public static const: string = 'const'
	public static let: string = 'let'
	public static warning: string = 'warning'
}

export function createDocumentationString(details: any): string {
	let documentationString = displayPartsToString(details.documentation)
	if (details.tags) {
		for (const tag of details.tags) {
			documentationString += `\n\n${tagToString(tag)}`
		}
	}
	return documentationString
}

function tagToString(tag: any): string {
	let tagLabel = `*@${tag.name}*`
	if (tag.name === 'param' && tag.text) {
		const [paramName, ...rest] = tag.text
		tagLabel += `\`${paramName.text}\``
		if (rest.length > 0) tagLabel += ` — ${rest.map((r) => r.text).join(' ')}`
	} else if (Array.isArray(tag.text)) {
		tagLabel += ` — ${tag.text.map((r) => r.text).join(' ')}`
	} else if (tag.text) {
		tagLabel += ` — ${tag.text}`
	}
	return tagLabel
}

export function displayPartsToString(displayParts: any | undefined): string {
	if (displayParts) {
		return displayParts.map((displayPart) => displayPart.text).join('')
	}
	return ''
}

// In the VSCode webview (iframe), Monaco's clipboard access is restricted:
// `navigator.clipboard.readText()` and the native `paste` event's
// `clipboardData` are both empty. The only reliable way to read the clipboard
// is to focus a real <input> and run `document.execCommand('paste')`.
//
// Registering paste via `editor.addCommand(Ctrl+V)` is also broken with
// multiple editors: standalone editors share a global keybinding registry, so
// the last-registered handler wins and paste lands in the wrong editor.
// Instead, scope a keydown listener to the editor's own container element and
// insert into whichever editor currently has text focus.
//
// No-op (returns an empty cleanup) when not running inside a webview iframe.
export function registerWebviewPaste(
	container: HTMLElement | null | undefined,
	getEditor: () => meditor.ICodeEditor | null | undefined
): () => void {
	if (!container || typeof window === 'undefined' || window.parent === window) {
		return () => {}
	}

	const input = document.createElement('input')
	input.type = 'text'
	input.tabIndex = -1
	input.setAttribute('aria-hidden', 'true')
	input.style.cssText = 'height:0;width:0;opacity:0;position:absolute;top:0;left:0;z-index:-1;'
	document.body.appendChild(input)

	const onInput = () => {
		const text = input.value
		input.value = ''
		if (!text) return
		const editor = getEditor()
		if (!editor) return
		// Bail rather than fall back to {1,1,1,1}: pasting at document start
		// would silently corrupt the document if the selection is ever lost.
		const selection = editor.getSelection()
		if (!selection) return
		editor.executeEdits('paste', [
			{
				range: selection,
				text,
				forceMoveMarkers: true
			}
		])
		editor.focus()
	}
	input.addEventListener('input', onInput)

	const onKeydown = (e: KeyboardEvent) => {
		const isPaste = (e.ctrlKey || e.metaKey) && !e.altKey && (e.key === 'v' || e.key === 'V')
		if (!isPaste) return
		const editor = getEditor()
		if (!editor || !editor.hasTextFocus()) return
		e.preventDefault()
		e.stopPropagation()
		input.focus()
		document.execCommand('paste')
	}
	container.addEventListener('keydown', onKeydown, true)

	return () => {
		container.removeEventListener('keydown', onKeydown, true)
		input.removeEventListener('input', onInput)
		input.remove()
	}
}
