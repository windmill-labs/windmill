import { editor as meditor } from 'monaco-editor'

/**
 * Unused for now but might be useful for alternative completion diff
 */
export class DiffEditorWidget {
	editor: any
	domNode: HTMLElement
	diffContainer: HTMLElement
	diffEditor: meditor.IStandaloneDiffEditor
	constructor(editor: meditor.IStandaloneCodeEditor, modified: string, lang: string) {
		this.editor = editor
		this.domNode = document.createElement('div')

		this.domNode.style.backgroundColor = 'var(--vscode-editor-background)'
		this.domNode.style.border = '1px solid #ccc'
		this.domNode.style.zIndex = '1000' // Make sure it's above other elements

		this.diffContainer = document.createElement('div')
		this.diffContainer.style.width = '100%'
		this.diffContainer.style.height = '100%'
		this.diffContainer.style.padding = '0'
		this.domNode.appendChild(this.diffContainer)

		// Create a diff editor inside the widget
		this.diffEditor = meditor.createDiffEditor(this.diffContainer, {
			readOnly: true,
			automaticLayout: true,
			lineNumbers: 'off',
			renderSideBySide: false,
			minimap: {
				enabled: false
			},
			scrollbar: {
				vertical: 'hidden',
				horizontal: 'hidden'
			},
			scrollBeyondLastLine: false,
			folding: false,
			glyphMargin: false,
			renderOverviewRuler: false,
			overviewRulerLanes: 0,
			renderIndicators: false,
			lineDecorationsWidth: 5,
			lightbulb: {
				enabled: meditor.ShowLightbulbIconMode.Off
			},
			lineNumbersMinChars: 0,
			renderMarginRevertIcon: false
		})

		const originalModel = meditor.createModel(editor.getValue() || '', lang)

		const modifiedModel = meditor.createModel(modified, lang)

		this.diffEditor.setModel({
			original: originalModel,
			modified: modifiedModel
		})

		function getMaxColumn(model: meditor.ITextModel) {
			if (!model) return 0

			let maxColumn = 0
			const totalLines = model.getLineCount()

			for (let line = 1; line <= totalLines; line++) {
				maxColumn = Math.max(maxColumn, model.getLineMaxColumn(line))
			}

			return maxColumn
		}

		const maxOriginal = getMaxColumn(originalModel)
		const maxModified = getMaxColumn(modifiedModel)
		const max = Math.max(maxOriginal, maxModified)
		const width = Math.min(max * 8, 600)
		this.domNode.style.width = `${width}px`
		this.diffEditor.onDidUpdateDiff(() => {
			const originalLineCount = originalModel.getLineCount()

			const changes = this.diffEditor.getLineChanges() || []

			console.log('changes', changes)

			let extraLines = 0

			console.log('original line count', originalLineCount)

			changes.forEach((change) => {
				if (change.modifiedEndLineNumber) {
					extraLines += change.modifiedEndLineNumber - change.modifiedStartLineNumber + 1
				}
			})

			const lines = originalLineCount + extraLines
			console.log('lines', lines)
			this.domNode.style.height = `${lines * 20}px`
		})
		// console.log(changes)
		// console.log('lineCount1', lineCount1)
		// console.log('lineCount2', lineCount2)
	}

	layout() {
		this.diffEditor.layout()
	}

	getId() {
		return 'diffEditorWidget'
	}

	getDomNode() {
		return this.domNode
	}

	getPosition() {
		return {
			position: {
				lineNumber: 1,
				column: 10000
			},
			preference: [meditor.ContentWidgetPositionPreference.EXACT]
		}
	}
}
