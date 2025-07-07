import { diffLines } from 'diff'
import { KeyCode, type IDisposable, type editor as meditor } from 'monaco-editor'
import {
	applyChange,
	displayVisualChanges,
	getLines,
	setGlobalCSS,
	type VisualChange
} from '../shared'
import { writable, type Writable } from 'svelte/store'
import { aiChatManager } from './AIChatManager.svelte'

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
	groupChanges: { changes: VisualChangeWithDiffIndex[]; groupIndex: number }[] = []

	constructor(editor: meditor.IStandaloneCodeEditor) {
		this.editor = editor
	}

	clear() {
		this.groupChanges = []
		aiChatManager.pendingNewCode = undefined
		for (const collection of this.decorationsCollections) {
			collection.clear()
		}
		this.editor.changeViewZones((acc) => {
			for (const id of this.viewZoneIds) {
				acc.removeZone(id)
			}
			this.viewZoneIds = []
		})
		setGlobalCSS('editor-windmill-chat-style', '')
	}
	preventWriting() {
		if (this.readOnlyDisposable) {
			this.readOnlyDisposable.dispose()
		}
		this.readOnlyDisposable = this.editor.onKeyDown((e) => {
			if ((e.ctrlKey || e.metaKey) && (e.keyCode === KeyCode.KeyZ || e.keyCode === KeyCode.KeyK)) {
				// allow undo/redo and cmd k
				return
			}
			e.preventDefault()
			e.stopPropagation()
		})
		this.editor.updateOptions({
			scrollBeyondLastLine: true
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
		this.editor.updateOptions({
			scrollBeyondLastLine: false
		})
	}

	async acceptAll() {
		this.groupChanges.reverse()
		for (const group of this.groupChanges) {
			this.applyGroup(group)
		}
		this.finish()
	}

	async rejectAll() {
		this.finish()
	}

	applyGroup(group: { changes: VisualChangeWithDiffIndex[]; groupIndex: number }) {
		// maximum of 2 changes per group with the deletion first
		if (group.changes.length > 2) {
			throw new Error('Invalid group')
		} else if (group.changes.length === 2) {
			const deletedChange = group.changes[0]
			const addedChange = group.changes[1]
			if (deletedChange.type === 'deleted' && addedChange.type === 'added_block') {
				applyChange(this.editor, deletedChange)
				addedChange.position.afterLineNumber = deletedChange.range.startLine - 1
				applyChange(this.editor, addedChange)
			} else {
				throw new Error('Invalid group')
			}
		} else if (group.changes.length === 1) {
			applyChange(this.editor, group.changes[0])
		}
	}

	private async calculateVisualChanges(newCode: string) {
		this.preventWriting()
		this.reviewingChanges.set(true)
		this.groupChanges = []
		const currentCode = this.editor.getValue()
		const changedLines = diffLines(currentCode, newCode)
		let visualChanges: VisualChangeWithDiffIndex[] = []

		let lineNumber = 1
		for (const [idx, change] of changedLines.entries()) {
			const nbOfNewLines = change.count || 1
			if (idx > 0 && changedLines[idx - 1].removed && !change.added) {
				this.groupChanges.push({ changes: visualChanges, groupIndex: this.groupChanges.length })
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
				this.groupChanges.push({ changes: visualChanges, groupIndex: this.groupChanges.length })
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
			this.groupChanges.push({ changes: visualChanges, groupIndex: this.groupChanges.length })
		}

		if (this.groupChanges.length === 0) {
			this.finish()
			return []
		}
		return changedLines
	}

	async reviewAndApply(newCode: string) {
		if (aiChatManager.pendingNewCode === newCode) {
			this.acceptAll()
			return
		} else if (aiChatManager.pendingNewCode) {
			this.clear()
		}
		aiChatManager.pendingNewCode = newCode
		const changedLines = await this.calculateVisualChanges(newCode)
		if (changedLines.length === 0) return

		let indicesOfRejectedLineChanges: number[] = []

		for (const [groupIndex, group] of this.groupChanges.entries()) {
			let collection: meditor.IEditorDecorationsCollection | undefined = undefined
			let ids: string[] = []
			const acceptFn = () => {
				this.applyGroup(group)
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
				indicesOfRejectedLineChanges.push(...group.changes.map((c) => c.diffIndex))
				collection?.clear()
				this.editor.changeViewZones((acc) => {
					for (const id of ids) {
						acc.removeZone(id)
					}
				})
				this.groupChanges = this.groupChanges.filter((g) => g.groupIndex !== groupIndex)
				if (this.groupChanges.length === 0) {
					this.finish()
				}
			}
			const changes = group.changes.map((c, i) => {
				if (i === group.changes.length - 1) {
					return {
						...c,
						options: { ...(c.options ?? {}), review: { acceptFn, rejectFn } }
					}
				} else {
					return c
				}
			})

			;({ collection, ids } = await displayVisualChanges(
				'editor-windmill-chat-style',
				this.editor,
				changes
			))
			this.decorationsCollections.push(collection)
			this.viewZoneIds.push(...ids)
		}
	}
}
