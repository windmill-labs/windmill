import { createLongHash } from '$lib/editorLangUtils'
import { type editor as meditor } from 'monaco-editor'

export type VisualChange =
	| {
		type: 'added_inline'
		position: {
			line: number
			column: number
		}
		value: string
		options?: {
			greenHighlight?: boolean
		}
	}
	| {
		type: 'added_block'
		position: {
			afterLineNumber: number
		}
		value: string
		options?: {
			greenHighlight?: boolean
			review?: {
				acceptFn: () => void
				rejectFn: () => void
			}
			extraChanges?: VisualChange[]
		}
	}
	| {
		type: 'deleted'
		range: {
			startLine: number
			startColumn: number
			endLine: number
			endColumn: number
		}
		options?: {
			isWholeLine?: boolean
			review?: {
				acceptFn: () => void
				rejectFn: () => void
			}
		}
	}

function applyMonacoStyles(targetEl: HTMLElement, greenHighlight?: boolean) {
	const computedStyles = window.getComputedStyle(
		document.querySelector('.monaco-editor .view-lines')!
	)
	Object.assign(targetEl.style, {
		fontFamily: computedStyles.fontFamily,
		fontSize: computedStyles.fontSize,
		lineHeight: computedStyles.lineHeight,
		color: 'gray',
		whiteSpace: 'pre'
	})
	if (greenHighlight) {
		targetEl.style.backgroundColor = 'var(--vscode-diffEditor-insertedTextBackground)'
	}
}

export function setGlobalCSS(id: string, cssCode: string) {
	let styleTag = document.getElementById(id)
	if (!styleTag) {
		styleTag = document.createElement('style')
		styleTag.id = id
		document.head.appendChild(styleTag)
	}
	styleTag.textContent = cssCode
}

function addInlineGhostText(change: Extract<VisualChange, { type: 'added_inline' }>) {
	const cssId = createLongHash()
	const decoration = {
		range: {
			startLineNumber: change.position.line,
			startColumn: change.position.column,
			endLineNumber: change.position.line,
			endColumn: change.position.column + change.value.length
		},
		options: {
			beforeContentClassName: `editor-ghost-text editor-ghost-text-content-${cssId} ${change.options?.greenHighlight ? 'editor-ghost-text-green' : ''
				}`
		}
	}

	const safeContent = change.value.replaceAll('"', '\\"')

	const css = `
.editor-ghost-text-content-${cssId}::before { 
content: "${safeContent}";
white-space: pre;
}`

	return { decoration, css }
}

function getReviewButtons(
	editor: meditor.IStandaloneCodeEditor,
	acceptFn: () => void,
	rejectFn: () => void
) {
	const { contentWidth, verticalScrollbarWidth } = editor.getLayoutInfo()
	const scrollLeft = editor.getScrollLeft()
	const reviewButtons = document.createElement('div')
	reviewButtons.classList.add('absolute', 'flex', 'flex-row', 'z-10', 'rounded')

	Object.assign(reviewButtons.style, {
		fontFamily: 'Inter',
		transform: 'translate(-100%, 100%)',
		left: `${contentWidth - verticalScrollbarWidth + scrollLeft}px`,
		bottom: '0'
	})
	editor.onDidLayoutChange((e) => {
		const scrollLeft = editor.getScrollLeft()
		reviewButtons.style.left = `${e.contentWidth - e.verticalScrollbarWidth + scrollLeft}px`
	})
	editor.onDidScrollChange((e) => {
		const { contentWidth, verticalScrollbarWidth } = editor.getLayoutInfo()
		reviewButtons.style.left = `${contentWidth - verticalScrollbarWidth + e.scrollLeft}px`
	})

	const acceptButton = document.createElement('button')
	acceptButton.textContent = 'Accept'
	Object.assign(acceptButton.style, {
		color: 'black',
		padding: '0.1rem 0.2rem',
		backgroundColor: 'rgb(160, 230, 160)'
	})
	acceptButton.classList.add('text-xs', 'font-normal', 'rounded-bl')
	acceptButton.addEventListener('click', () => {
		acceptFn()
	})
	const layout = editor.getLayoutInfo()
	layout.width
	const rejectButton = document.createElement('button')
	rejectButton.textContent = 'Reject'
	Object.assign(rejectButton.style, {
		color: 'black',
		padding: '0.1rem 0.2rem',
		backgroundColor: 'rgb(230, 160, 160)'
	})
	rejectButton.classList.add('text-xs', 'font-normal', 'rounded-br')
	rejectButton.addEventListener('click', () => {
		rejectFn()
	})
	reviewButtons.append(acceptButton)
	reviewButtons.append(rejectButton)
	return reviewButtons
}

async function addMultilineGhostText(
	editor: meditor.IStandaloneCodeEditor,
	text: string,
	afterLineNumber: number,
	heightInLines: number,
	options?: {
		greenHighlight?: boolean
		review?: {
			acceptFn: () => void
			rejectFn: () => void
		}
		extraChanges?: VisualChange[]
	}
) {
	const el = document.createElement('div')
	el.textContent = text

	if (options?.review) {
		const reviewButtons = getReviewButtons(editor, options.review.acceptFn, options.review.rejectFn)
		el.append(reviewButtons)
	}
	applyMonacoStyles(el, options?.greenHighlight)
	const addZonePromise = new Promise<string>((resolve, reject) => {
		editor?.changeViewZones((acc) => {
			const id = acc.addZone({
				afterLineNumber,
				afterColumn: 0,
				heightInLines,
				domNode: el
			})
			resolve(id)
		})
	})

	return addZonePromise
}

export let VISUAL_CHANGES_CSS = `.editor-ghost-text-green { background-color: var(--vscode-diffEditor-insertedTextBackground) !important; }\n.editor-ghost-text-removed { background-color: var(--vscode-diffEditor-removedTextBackground); }\n\n.editor-ghost-text { display: inline-block; background-color: var(--vscode-editor-background); color: gray;}`

export async function displayVisualChanges(
	cssId: string,
	editor: meditor.IStandaloneCodeEditor,
	visualChanges: VisualChange[]
) {
	let decorations: meditor.IModelDeltaDecoration[] = []
	let css = ''
	let ids: string[] = []
	for (const change of visualChanges) {
		if (change.type === 'added_inline') {
			const { css: newCss, decoration } = addInlineGhostText(change)
			decorations.push(decoration)
			css += newCss
		} else if (change.type === 'deleted') {
			const decoration: meditor.IModelDeltaDecoration = {
				range: {
					startLineNumber: change.range.startLine,
					startColumn: change.range.startColumn,
					endLineNumber: change.range.endLine,
					endColumn: change.range.endColumn
				},
				options: {
					className: 'editor-ghost-text-removed',
					isWholeLine: change.options?.isWholeLine
				}
			}
			if (change.options?.review) {
				const id = await new Promise<string>((resolve, reject) => {
					editor.changeViewZones((acc) => {
						if (change.options?.review) {
							const el = document.createElement('div')
							const reviewButtons = getReviewButtons(
								editor,
								change.options.review.acceptFn,
								change.options.review.rejectFn
							)
							el.append(reviewButtons)
							resolve(
								acc.addZone({
									afterLineNumber: change.range.endLine,
									afterColumn: 0,
									heightInLines: 0,
									domNode: el
								})
							)
						}
					})
				})
				ids.push(id)
			}
			decorations.push(decoration)
		} else if (change.type === 'added_block') {
			const id = await addMultilineGhostText(
				editor,
				change.value,
				change.position.afterLineNumber,
				change.value.split('\n').length, // we know it won't end by \n
				change.options
			)
			ids.push(id)
		}
	}
	const collection = editor.createDecorationsCollection(decorations)

	setGlobalCSS(cssId, VISUAL_CHANGES_CSS + css)
	return { collection, ids }
}

export function applyChange(editor: meditor.IStandaloneCodeEditor, change: VisualChange) {
	if (change.type === 'added_block') {
		editor.executeEdits('chat', [
			{
				range: {
					startLineNumber: change.position.afterLineNumber + 1,
					startColumn: 0,
					endLineNumber: change.position.afterLineNumber + 1,
					endColumn: 1
				},
				text: change.value + '\n'
			}
		])
	} else if (change.type === 'deleted') {
		editor.executeEdits('chat', [
			{
				range: {
					startLineNumber: change.range.startLine,
					startColumn: change.range.startColumn,
					endLineNumber: change.range.endLine + 1,
					endColumn: 0
				},
				text: ''
			}
		])
	}
}

export function getLines(code: string) {
	const lines = code.split('\n')
	if (code.endsWith('\n')) {
		lines.pop()
	}
	return lines
}
