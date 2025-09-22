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

export interface ReviewChangesOpts {
	applyAll?: boolean
	mode?: 'apply' | 'revert'
	onFinishedReview?: () => void
}

export class AIChatEditorHandler {
	editor: meditor.IStandaloneCodeEditor
	viewZoneIds: string[] = []
	decorationsCollections: meditor.IEditorDecorationsCollection[] = []
	readOnlyDisposable: IDisposable | undefined = undefined

	reviewingChanges: Writable<boolean> = writable(false)
	groupChanges: { changes: VisualChangeWithDiffIndex[]; groupIndex: number }[] = []

	// Track review decisions
	private reviewState: {
		mode: 'apply' | 'revert'
		onFinishedReview?: () => void
	} | null = null

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

	async finish(opts?: { disableReviewCallback?: boolean }) {
		// expose mode getter relies on reviewState
		// Call completion callback if we're tracking review state
		if (this.reviewState?.onFinishedReview && !opts?.disableReviewCallback) {
			this.reviewState.onFinishedReview()
		}

		// Reset review state
		this.reviewState = null

		this.clear()
		this.allowWriting()
		this.reviewingChanges.set(false)
		this.editor.updateOptions({
			scrollBeyondLastLine: false
		})
	}

	getReviewMode(): 'apply' | 'revert' | null {
		return this.reviewState?.mode ?? null
	}

	async acceptAll(opts?: { disableReviewCallback?: boolean }) {
		this.groupChanges.reverse()
		for (const group of this.groupChanges) {
			this.applyGroup(group)
		}
		this.finish(opts)
	}

	async rejectAll(opts?: { disableReviewCallback?: boolean }) {
		this.finish(opts)
	}

	// Keep all changes, used in revert mode
	async keepAll(opts?: { disableReviewCallback?: boolean }) {
		this.finish(opts)
	}

	// Revert all changes, used in revert mode
	async revertAll(opts?: { disableReviewCallback?: boolean }) {
		this.acceptAll(opts)
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

	async reviewChanges(targetCode: string, opts?: ReviewChangesOpts) {
		if (aiChatManager.pendingNewCode === targetCode && opts?.mode === 'apply') {
			this.acceptAll()
			return
		} else if (aiChatManager.pendingNewCode) {
			this.clear()
		}

		aiChatManager.pendingNewCode = targetCode
		const changedLines = await this.calculateVisualChanges(targetCode)
		if (changedLines.length === 0) return

		// Initialize review state for tracking
		this.reviewState = {
			mode: opts?.mode ?? 'apply',
			onFinishedReview: opts?.onFinishedReview
		}

		let indicesOfRejectedLineChanges: number[] = []

		for (const [groupIndex, group] of this.groupChanges.entries()) {
			let collection: meditor.IEditorDecorationsCollection | undefined = undefined
			let ids: string[] = []

			const isRevert = opts?.mode === 'revert'

			// Apply this group and continue with remaining changes
			const onApply = () => {
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
				this.reviewChanges(newCodeWithRejects, opts)
			}

			// Discard this group and continue with remaining changes
			const onDiscard = () => {
				// This group was not applied (not reverted in revert mode)
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

			// In revert mode: Accept = keep current code, Reject = revert to targetCode
			// In apply mode: Accept = apply changes, Reject = discard changes
			const acceptFn = isRevert ? onDiscard : onApply
			const rejectFn = isRevert ? onApply : onDiscard

			const changes = group.changes.map((c, i) => {
				if (i === group.changes.length - 1) {
					return {
						...c,
						options: {
							...(c.options ?? {}),
							review: {
								acceptFn,
								rejectFn
							}
						}
					}
				} else {
					return c
				}
			})

			if (!opts?.applyAll) {
				;({ collection, ids } = await displayVisualChanges(
					'editor-windmill-chat-style',
					this.editor,
					changes,
					isRevert
				))
				this.decorationsCollections.push(collection)
				this.viewZoneIds.push(...ids)
			}
		}
		if (opts?.applyAll) {
			this.acceptAll()
		}
	}
}
