import { type Change, createTwoFilesPatch, diffLines, diffWordsWithSpace } from 'diff'
import { type editor as meditor } from 'monaco-editor'
import { autocompleteRequest } from './request'
import { sleep } from '$lib/utils'

function applyMonacoStyles(targetEl: HTMLElement, greenHighlight?: boolean) {
	const computedStyles = window.getComputedStyle(
		document.querySelector('.monaco-editor .view-lines')!
	)
	targetEl.style.fontFamily = computedStyles.fontFamily
	targetEl.style.fontSize = computedStyles.fontSize
	targetEl.style.lineHeight = computedStyles.lineHeight
	targetEl.style.color = 'gray'
	if (greenHighlight) {
		targetEl.style.backgroundColor = 'var(--vscode-diffEditor-insertedTextBackground)'
	}
	targetEl.style.whiteSpace = 'pre' // Preserve spacing like Monaco
}

function setAutocompleteGlobalCSS(cssCode: string) {
	let styleTag = document.getElementById('editor-windmill-autocomplete-style')

	if (!styleTag) {
		styleTag = document.createElement('style')
		styleTag.id = 'editor-windmill-autocomplete-style'
		document.head.appendChild(styleTag)
	}

	styleTag.innerHTML = cssCode
}

function addInlineGhostText(text: string, line: number, col: number, isReplacing: boolean) {
	const cssId = crypto.randomUUID()
	const decoration = {
		range: {
			startLineNumber: line,
			startColumn: col,
			endLineNumber: line,
			endColumn: col
		},
		options: {
			beforeContentClassName: `editor-ghost-text editor-ghost-text-content-${cssId} ${
				isReplacing ? 'editor-ghost-text-replaced' : ''
			}`
		}
	}

	const safeContent = text.replaceAll('"', '\\"')

	const css = `
.editor-ghost-text-content-${cssId}::before { 
content: "${safeContent}";
white-space: pre;
}`

	return { decoration, css }
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

function getReviewButtons(
	editor: meditor.IStandaloneCodeEditor,
	acceptFn: () => void,
	rejectFn: () => void
) {
	const { contentWidth, verticalScrollbarWidth } = editor.getLayoutInfo()
	const scrollLeft = editor.getScrollLeft()
	const reviewButtons = document.createElement('div')
	reviewButtons.classList.add('absolute', 'flex', 'flex-row', 'z-10', 'rounded')
	reviewButtons.style.fontFamily = 'Inter'
	reviewButtons.style.transform = 'translate(-100%, 100%)'
	reviewButtons.style.left = `${contentWidth - verticalScrollbarWidth + scrollLeft}px`
	reviewButtons.style.bottom = '0'
	editor.onDidLayoutChange((e) => {
		const scrollLeft = editor.getScrollLeft()
		reviewButtons.style.left = `${e.contentWidth - e.verticalScrollbarWidth + scrollLeft}px`
	})
	editor.onDidScrollChange((e) => {
		const { contentWidth, verticalScrollbarWidth } = editor.getLayoutInfo()
		reviewButtons.style.left = `${contentWidth - verticalScrollbarWidth + e.scrollLeft}px`
	})

	const acceptButton = document.createElement('button')
	acceptButton.innerHTML = 'Accept'
	acceptButton.style.color = 'black'
	acceptButton.style.padding = '0.1rem 0.2rem'
	acceptButton.style.backgroundColor = 'rgb(160, 230, 160)'
	acceptButton.classList.add('text-xs', 'font-normal', 'rounded-bl')
	acceptButton.addEventListener('click', () => {
		acceptFn()
	})
	const layout = editor.getLayoutInfo()
	layout.width
	const rejectButton = document.createElement('button')
	rejectButton.innerHTML = 'Reject'
	rejectButton.style.color = 'black'
	rejectButton.style.padding = '0.1rem 0.2rem'
	rejectButton.style.backgroundColor = 'rgb(230, 160, 160)'
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
	el.innerHTML = text

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

export type VisualChange =
	| {
			type: 'added_inline'
			position: {
				line: number
				column: number
			}
			value: string
			isReplacing: boolean
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

function lineChangesToVisualChanges(changes: Change[], startLineNumber: number) {
	let originalLineNumber = startLineNumber
	let visualChanges: VisualChange[] = []

	let removedLines: string[] = []

	for (const c of changes) {
		if (c.removed) {
			const lines = getLines(c.value)
			originalLineNumber += lines.length
			removedLines.push(...lines)
		} else if (c.added) {
			const newLines = getLines(c.value)
			const removedStartLineNumber = originalLineNumber - removedLines.length
			let afterLines: string[] = []
			for (const [idx, newLine] of newLines.entries()) {
				const originalLine = removedLines[idx]
				if (originalLine !== undefined) {
					const lineDiff = diffWordsWithSpace(originalLine, newLine)
					const firstRemovedChangeIdx = lineDiff.findIndex((c) => c.removed)
					if (firstRemovedChangeIdx !== -1 && lineDiff.length > 3) {
						let startColumn = 1
						let newLineContent = newLine
						const firstChange = lineDiff[0]
						if (
							!firstChange.added &&
							!firstChange.removed &&
							firstChange.value.trim().length === 0
						) {
							startColumn += firstChange.value.length
							newLineContent = newLineContent.slice(firstChange.value.length)
						}
						visualChanges.push({
							type: 'deleted',
							range: {
								startLine: removedStartLineNumber + idx,
								startColumn,
								endLine: removedStartLineNumber + idx,
								endColumn: 10000
							}
						})
						visualChanges.push({
							type: 'added_inline',
							position: {
								line: removedStartLineNumber + idx,
								column: 10000
							},
							value: newLineContent,
							isReplacing: true
						})
					} else {
						let col = 1
						let removedChars = 0
						for (const charChange of lineDiff) {
							if (charChange.added) {
								visualChanges.push({
									type: 'added_inline',
									position: {
										line: removedStartLineNumber + idx,
										column: col
									},
									value: charChange.value,
									isReplacing: removedChars > 0
								})
								removedChars = Math.max(0, removedChars - charChange.value.length)
							} else if (charChange.removed) {
								visualChanges.push({
									type: 'deleted',
									range: {
										startLine: removedStartLineNumber + idx,
										startColumn: col,
										endLine: removedStartLineNumber + idx,
										endColumn: col + charChange.value.length
									}
								})
								removedChars += charChange.value.length
								col += charChange.value.length
							} else {
								col += charChange.value.length
								removedChars = 0
							}
						}
					}
				} else {
					afterLines.push(newLine)
				}
			}
			if (afterLines.length > 0) {
				visualChanges.push({
					type: 'added_block',
					position: {
						afterLineNumber: originalLineNumber - 1
					},
					value: afterLines.join('\n')
				})
			}
			if (removedLines.length > newLines.length) {
				for (let i = 0; i < removedLines.length - newLines.length; i++) {
					visualChanges.push({
						type: 'deleted',
						range: {
							startLine: removedStartLineNumber + newLines.length + i,
							startColumn: 0,
							endLine: removedStartLineNumber + newLines.length + i,
							endColumn: 100000
						}
					})
				}
			}
			removedLines = []
		} else {
			if (removedLines.length > 0) {
				visualChanges.push({
					type: 'deleted',
					range: {
						startLine: originalLineNumber - removedLines.length,
						startColumn: 0,
						endLine: originalLineNumber - 1,
						endColumn: 10000
					}
				})
			}
			originalLineNumber += c.count!
			removedLines = []
		}
	}
	if (removedLines.length > 0) {
		visualChanges.push({
			type: 'deleted',
			range: {
				startLine: originalLineNumber - removedLines.length,
				startColumn: 0,
				endLine: originalLineNumber - 1,
				endColumn: 10000
			}
		})
	}
	return visualChanges
}

export let VISUAL_CHANGES_CSS = `.editor-ghost-text-replaced { background-color: var(--vscode-diffEditor-insertedTextBackground) !important; }\n.editor-ghost-text-removed { background-color: var(--vscode-diffEditor-removedTextBackground); }\n\n.editor-ghost-text { display: inline-block; background-color: var(--vscode-editor-background); color: gray;}`

export async function displayVisualChanges(
	editor: meditor.IStandaloneCodeEditor,
	visualChanges: VisualChange[]
) {
	let decorations: meditor.IModelDeltaDecoration[] = []
	let css = ''
	let ids: string[] = []
	for (const change of visualChanges) {
		if (change.type === 'added_inline') {
			const { css: newCss, decoration } = addInlineGhostText(
				change.value,
				change.position.line,
				change.position.column,
				change.isReplacing
			)
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

	setAutocompleteGlobalCSS(VISUAL_CHANGES_CSS + css)
	return { collection, ids }
}

export function getLines(code: string) {
	const lines = code.split('\n')
	if (code.endsWith('\n')) {
		lines.pop()
	}
	return lines
}

const MAX_PATCHES = 4

export class Autocompletor {
	editor: meditor.IStandaloneCodeEditor
	language: string

	viewZoneIds: string[] = []
	decorationsCollection: meditor.IEditorDecorationsCollection | undefined = undefined
	visualChanges: VisualChange[] = []
	modifiedCode: string = ''
	applyZone:
		| {
				startLineNumber: number
				endLineNumber: number
		  }
		| undefined = undefined
	lastChangePosition:
		| {
				lineNumber: number
				column: number
		  }
		| undefined = undefined

	abortController: AbortController | undefined = undefined
	lastTs = Date.now()

	lastCodeValue: string
	patches: string[] = []

	predictedChange:
		| {
				position: {
					lineNumber: number
					column: number
				}
				distance: number
		  }
		| undefined = undefined
	tabWidget: meditor.IContentWidget | undefined = undefined

	constructor(editor: meditor.IStandaloneCodeEditor, language: string) {
		this.editor = editor
		this.language = language
		this.lastCodeValue = editor.getModel()?.getValue() || ''
	}

	savePatch() {
		const currentCode = this.editor.getModel()?.getValue() || ''
		const patch = createTwoFilesPatch(
			'',
			'',
			this.lastCodeValue,
			currentCode,
			undefined,
			undefined,
			{
				context: 1
			}
		)
			.split('\n')
			.slice(4)
			.join('\n')

		this.patches.push(patch)
		this.lastCodeValue = currentCode
		if (this.patches.length > MAX_PATCHES) {
			this.patches.shift()
		}
	}

	async predict() {
		this.reject()
		await this.autocomplete()
		this.computeNextPosition()
		this.displayPrediction()
	}

	computeNextPosition() {
		if (this.visualChanges.length > 0) {
			const position = this.editor.getPosition()
			if (!position) {
				return
			}

			let closestPosition:
				| {
						lineNumber: number
						column: number
				  }
				| undefined = undefined
			let closestDistance = Infinity
			for (const change of this.visualChanges) {
				if (change.type === 'deleted') {
					const distance = Math.min(
						Math.abs(change.range.startLine - position.lineNumber) +
							Math.abs(change.range.startColumn - position.column) / 10000,
						Math.abs(change.range.endLine - position.lineNumber) +
							Math.abs(change.range.endColumn - position.column) / 10000
					)
					if (distance < closestDistance) {
						closestDistance = distance
						closestPosition = {
							lineNumber: change.range.startLine,
							column: change.range.startColumn
						}
					}
				} else if (change.type === 'added_block') {
					const distance = Math.abs(change.position.afterLineNumber - position.lineNumber) + 1
					if (distance < closestDistance) {
						closestDistance = distance
						closestPosition = {
							lineNumber: change.position.afterLineNumber,
							column: 10000
						}
					}
				} else if (change.type === 'added_inline') {
					const distance =
						Math.abs(change.position.line - position.lineNumber) +
						Math.abs(change.position.column - position.column) / 10000
					if (distance < closestDistance) {
						closestDistance = distance
						closestPosition = {
							lineNumber: change.position.line,
							column: change.position.column
						}
					}
				}
			}
			this.predictedChange = closestPosition
				? { position: closestPosition, distance: closestDistance }
				: undefined

			console.log('predictedChange', this.predictedChange, this.visualChanges)
		}
	}

	displayPrediction() {
		if (this.predictedChange) {
			if (this.predictedChange.distance < 4) {
				this.predictedChange = undefined
				this.displayVisualChanges()
			} else {
				// display tab icon
				const el = document.createElement('div')
				el.innerHTML = 'TAB'

				Object.assign(el.style, {
					position: 'relative',
					background: '#e7e5e4',
					color: 'black',
					padding: '4px',
					fontSize: '10px',
					borderRadius: '4px',
					textAlign: 'center',
					transform: 'translateX(-50%)',
					zIndex: 1000,
					opacity: 0.8
				})

				// Create the arrow (pseudo-element trick doesn't work directly via JS,
				// so we create a separate element to act like the arrow)
				const arrow = document.createElement('div')
				Object.assign(arrow.style, {
					content: '""',
					position: 'absolute',
					top: '-6px',
					left: '50%',
					transform: 'translateX(-50%)',
					width: '0',
					height: '0',
					borderLeft: '6px solid transparent',
					borderRight: '6px solid transparent',
					borderBottom: '6px solid #e7e5e4'
				})

				// Add arrow to box
				el.appendChild(arrow)
				this.tabWidget = {
					getId: () => 'tab-widget',
					getDomNode: () => el,
					getPosition: () => {
						if (!this.predictedChange) {
							return null
						}
						return {
							position: {
								lineNumber: this.predictedChange.position.lineNumber,
								column: this.predictedChange.position.column
							},
							preference: [2] // below
						}
					},
					allowEditorOverflow: true
				}
				this.editor.addContentWidget(this.tabWidget)
			}
		}
	}

	async autocomplete() {
		const position = this.editor.getPosition()
		if (!position) {
			return
		}

		const model = this.editor.getModel()

		if (!model) {
			return
		}

		const thisTs = Date.now()
		this.lastTs = thisTs

		await sleep(200)

		if (model.isDisposed()) {
			return
		}

		if (thisTs !== this.lastTs) {
			return
		}

		this.abortController?.abort()
		this.abortController = new AbortController()

		let modifiableEnd = Math.min(model.getLineCount(), position.lineNumber + 7)
		while (true) {
			if (modifiableEnd <= position.lineNumber) {
				break
			}
			const line = model.getLineContent(modifiableEnd)
			if (line.trim().length > 0) {
				break
			}
			modifiableEnd--
		}

		let modifiableStart = Math.max(1, position.lineNumber - 3)
		while (true) {
			if (modifiableStart >= modifiableEnd) {
				break
			}
			const line = model.getLineContent(modifiableStart)
			if (line.trim().length > 0) {
				break
			}
			modifiableStart++
		}

		const newCursorLineNumber = Math.max(position.lineNumber, modifiableStart)
		const newPos = {
			lineNumber: newCursorLineNumber,
			column: newCursorLineNumber === position.lineNumber ? position.column : 0
		}

		this.applyZone = {
			startLineNumber: modifiableStart,
			endLineNumber: modifiableEnd
		}

		const prefix = model.getValueInRange({
			startLineNumber: 1,
			startColumn: 1,
			endLineNumber: modifiableStart,
			endColumn: 1
		})

		const suffix = model.getValueInRange({
			startLineNumber: modifiableEnd + 1,
			startColumn: 0,
			endLineNumber: model.getLineCount(),
			endColumn: 10000
		})

		const modifiablePrefix = model.getValueInRange({
			startLineNumber: modifiableStart,
			startColumn: 1,
			endLineNumber: newPos.lineNumber,
			endColumn: newPos.column
		})

		const modifiableSuffix = model.getValueInRange({
			startLineNumber: newPos.lineNumber,
			startColumn: newPos.column,
			endLineNumber: modifiableEnd,
			endColumn: 10000
		})

		const returnedCode = await autocompleteRequest(
			{
				prefix,
				modifiablePrefix,
				modifiableSuffix,
				suffix,
				language: this.language,
				events: this.patches
			},
			this.abortController
		)

		if (!returnedCode) {
			return
		}

		const editableCode = model.getValueInRange({
			startLineNumber: modifiableStart,
			startColumn: 1,
			endLineNumber: modifiableEnd,
			endColumn: 10000
		})
		const numberOfLines = modifiableEnd - modifiableStart + 1

		let completionLines = getLines(returnedCode)

		let finalCompletionLines: string[] = []
		if (completionLines.length > numberOfLines) {
			const nextFirstNonEmptyLine = suffix.split('\n').find((line) => line.trim().length > 8)
			if (nextFirstNonEmptyLine) {
				for (const line of completionLines) {
					if (line === nextFirstNonEmptyLine) {
						break
					} else {
						finalCompletionLines.push(line)
					}
				}
			} else {
				finalCompletionLines = completionLines
			}
		} else {
			finalCompletionLines = completionLines
		}
		this.modifiedCode = finalCompletionLines.join('\n')

		const changedLines = diffLines(editableCode, this.modifiedCode)

		this.visualChanges = lineChangesToVisualChanges(changedLines, modifiableStart)
	}

	async displayVisualChanges() {
		if (this.visualChanges.length > 0) {
			const { collection, ids } = await displayVisualChanges(this.editor, this.visualChanges)
			this.decorationsCollection = collection
			this.viewZoneIds = ids

			const lastAddChange = this.visualChanges
				.reverse()
				.find((c) => c.type === 'added_inline' || c.type === 'added_block')
			if (lastAddChange) {
				if (lastAddChange.type === 'added_inline') {
					this.lastChangePosition = {
						lineNumber: lastAddChange.position.line,
						column: lastAddChange.position.column + lastAddChange.value.length
					}
				} else if (lastAddChange.type === 'added_block') {
					this.lastChangePosition = {
						lineNumber:
							lastAddChange.position.afterLineNumber + lastAddChange.value.split('\n').length,
						column: 10000
					}
				}
			}
		}
	}

	hasChanges() {
		return this.modifiedCode.length > 0
	}

	accept() {
		if (this.predictedChange) {
			this.editor.setPosition(this.predictedChange.position)
		}

		if (!this.modifiedCode || !this.applyZone) {
			return
		}

		this.editor.executeEdits('completion', [
			{
				range: {
					startLineNumber: this.applyZone.startLineNumber,
					startColumn: 1,
					endLineNumber: this.applyZone.endLineNumber,
					endColumn: 10000
				},
				text: this.modifiedCode
			}
		])
		if (this.lastChangePosition) {
			this.editor.setPosition(this.lastChangePosition)
		}
		this.reject()
	}

	reject() {
		this.abortController?.abort()
		this.editor.changeViewZones((acc) => {
			for (const id of this.viewZoneIds) {
				acc.removeZone(id)
			}
			this.viewZoneIds = []
		})
		this.decorationsCollection?.clear()
		this.modifiedCode = ''
		setAutocompleteGlobalCSS('')
		this.predictedChange = undefined
		this.tabWidget && this.editor.removeContentWidget(this.tabWidget)
		this.tabWidget = undefined
		this.visualChanges = []
	}
}
