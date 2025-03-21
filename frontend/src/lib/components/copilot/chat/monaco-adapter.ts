import { diffLines } from 'diff'
import { KeyCode, type IDisposable, type editor as meditor } from 'monaco-editor'
import {
	applyChange,
	displayVisualChanges,
	getLines,
	type VisualChange
} from '../autocomplete/monaco-adapter'
import { writable, type Writable } from 'svelte/store'

type ExcludeVariant<T, K extends keyof T, V> = T extends Record<K, V> ? never : T
type VisualChangeWithDiffIndex = ExcludeVariant<VisualChange, 'type', 'added_inline'> & {
	diffIndex: number
}

export class AIChatEditorHandler {
	editor: meditor.IStandaloneCodeEditor
	viewZoneIds: string[] = []
	decorationsCollections: meditor.IEditorDecorationsCollection[] = []
	readOnlyDisposable: IDisposable | undefined = undefined

	reviewingChanges: Writable<boolean> = writable(false)

	groupChanges: VisualChangeWithDiffIndex[][] = []

	constructor(editor: meditor.IStandaloneCodeEditor) {
		this.editor = editor
	}

	clear() {
		this.groupChanges = []
		for (const collection of this.decorationsCollections) {
			collection.clear()
		}
		this.editor.changeViewZones((acc) => {
			for (const id of this.viewZoneIds) {
				acc.removeZone(id)
			}
			this.viewZoneIds = []
		})
	}
	preventWriting() {
		if (this.readOnlyDisposable) {
			this.readOnlyDisposable.dispose()
		}
		this.readOnlyDisposable = this.editor.onKeyDown((e) => {
			if ((e.ctrlKey || e.metaKey) && e.keyCode === KeyCode.KeyZ) {
				// allow undo/redo
				return
			}
			e.preventDefault()
			e.stopPropagation()
		})
	}

	allowWriting() {
		if (this.readOnlyDisposable) {
			this.readOnlyDisposable.dispose()
		}
	}

	async finish() {
		this.clear()
		this.allowWriting()
		this.reviewingChanges.set(false)
	}

	async acceptAll() {
		this.groupChanges.reverse()
		for (const group of this.groupChanges) {
			group.reverse()
			for (const change of group) {
				applyChange(this.editor, change)
			}
		}
		this.finish()
	}

	async rejectAll() {
		this.finish()
	}

	async reviewAndApply(newCode: string) {
		this.preventWriting()
		this.reviewingChanges.set(true)
		const currentCode = this.editor.getValue()
		const changedLines = diffLines(currentCode, newCode)

		this.groupChanges = []
		let visualChanges: VisualChangeWithDiffIndex[] = []

		let lineNumber = 1
		for (const [idx, change] of changedLines.entries()) {
			const nbOfNewLines = change.count || 1
			if (idx > 0 && changedLines[idx - 1].removed && !change.added) {
				this.groupChanges.push(visualChanges)
				visualChanges = []
			}
			if (change.added) {
				const lines = getLines(change.value)
				visualChanges.push({
					type: 'added_block',
					position: {
						afterLineNumber: lineNumber - 1
					},
					value: lines.join('\n'),
					options: {
						greenHighlight: true
					},
					diffIndex: idx
				})
				this.groupChanges.push(visualChanges)
				visualChanges = []
			} else if (change.removed) {
				visualChanges = []
				visualChanges.push({
					type: 'deleted',
					range: {
						startLine: lineNumber,
						startColumn: 1,
						endLine: lineNumber + (nbOfNewLines - 1),
						endColumn: 10000
					},
					options: {
						isWholeLine: true
					},
					diffIndex: idx
				})

				lineNumber += nbOfNewLines
			} else {
				lineNumber += nbOfNewLines
			}
		}
		if (visualChanges.length > 0) {
			this.groupChanges.push(visualChanges)
		}

		if (this.groupChanges.length === 0) {
			this.finish()
			return
		}

		let indicesOfRejectedLineChanges: number[] = []

		for (const [groupIndex, group] of this.groupChanges.entries()) {
			let collection: meditor.IEditorDecorationsCollection | undefined = undefined
			let ids: string[] = []
			const acceptFn = () => {
				group.reverse()
				for (const change of group) {
					applyChange(this.editor, change)
				}
				this.clear()
				let newCodeWithRejects = ''
				for (const [idx, change] of changedLines.entries()) {
					if (!change.added && !change.removed) {
						newCodeWithRejects += change.value
					} else if (change.added && !indicesOfRejectedLineChanges.includes(idx)) {
						newCodeWithRejects += change.value
					} else if (change.removed && indicesOfRejectedLineChanges.includes(idx)) {
						newCodeWithRejects += change.value
					}
				}
				this.reviewAndApply(newCodeWithRejects)
			}
			const rejectFn = () => {
				indicesOfRejectedLineChanges.push(...group.map((c) => c.diffIndex))
				collection?.clear()
				this.editor.changeViewZones((acc) => {
					for (const id of ids) {
						acc.removeZone(id)
					}
				})
				this.groupChanges.splice(groupIndex, 1)
				if (this.groupChanges.length === 0) {
					this.finish()
				}
			}
			const changes = group.map((c, i) => {
				if (i === group.length - 1) {
					return {
						...c,
						options: { ...(c.options ?? {}), review: { acceptFn, rejectFn } }
					}
				} else {
					return c
				}
			})

			;({ collection, ids } = await displayVisualChanges(this.editor, changes))
			this.decorationsCollections.push(collection)
			this.viewZoneIds.push(...ids)
		}
	}
}
