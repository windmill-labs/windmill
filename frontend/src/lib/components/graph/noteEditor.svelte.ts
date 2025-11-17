import type { FlowNote } from '$lib/gen'
import type { StateStore } from '$lib/utils'
import type { ExtendedOpenFlow } from '../flows/types'
import type { NoteColor } from './noteColors'
import { generateId } from './util'
import { getContext, setContext } from 'svelte'
import { completeAndSplitGroup } from './groupDetectionUtils'

/**
 * Utility class for editing flow notes via direct flowStore mutations
 * This class is designed to be used in editor contexts via Svelte context
 */
export class NoteEditor {
	private flowStore: StateStore<ExtendedOpenFlow>

	constructor(flowStore: StateStore<ExtendedOpenFlow>) {
		this.flowStore = flowStore
	}

	/**
	 * Get the current notes array from the flow store
	 */
	private getNotes(): FlowNote[] {
		return this.flowStore.val.value?.notes || []
	}

	/**
	 * Set the notes array in the flow store
	 */
	private setNotes(notes: FlowNote[]): void {
		if (this.flowStore.val.value) {
			this.flowStore.val.value.notes = notes
		}
	}

	/**
	 * Add a new note to the flow
	 */
	addNote(note: Omit<FlowNote, 'id'>): string {
		const notes = this.getNotes()
		const newNote: FlowNote = {
			id: generateId(),
			...note
		}
		this.setNotes([...notes, newNote])
		return newNote.id
	}

	/**
	 * Update the text content of a note
	 */
	updateText(noteId: string, text: string): void {
		const notes = this.getNotes()
		const updatedNotes = notes.map((note) => (note.id === noteId ? { ...note, text } : note))
		this.setNotes(updatedNotes)
	}

	/**
	 * Update the color of a note
	 */
	updateColor(noteId: string, color: NoteColor): void {
		const notes = this.getNotes()
		const updatedNotes = notes.map((note) => (note.id === noteId ? { ...note, color } : note))
		this.setNotes(updatedNotes)
	}

	/**
	 * Update the position of a note
	 */
	updatePosition(noteId: string, position: { x: number; y: number }): void {
		const notes = this.getNotes()
		const updatedNotes = notes.map((note) => (note.id === noteId ? { ...note, position } : note))
		this.setNotes(updatedNotes)
	}

	/**
	 * Update the size of a note
	 */
	updateSize(noteId: string, size: { width: number; height: number }): void {
		const notes = this.getNotes()
		const updatedNotes = notes.map((note) => (note.id === noteId ? { ...note, size } : note))
		this.setNotes(updatedNotes)
	}

	/**
	 * Toggle the locked state of a note
	 */
	updateLock(noteId: string, locked: boolean): void {
		const notes = this.getNotes()
		const updatedNotes = notes.map((note) => (note.id === noteId ? { ...note, locked } : note))
		this.setNotes(updatedNotes)
	}

	/**
	 * Delete a note from the flow
	 */
	deleteNote(noteId: string): void {
		const notes = this.getNotes()
		const updatedNotes = notes.filter((note) => note.id !== noteId)
		this.setNotes(updatedNotes)
	}

	/**
	 * Create a group note containing the specified node IDs
	 */
	createGroupNote(
		nodeIds: string[],
		text: string = '### Group note\nDouble click to edit me'
	): string {
		// Position and size will be calculated dynamically by layout

		const groupNote: Omit<FlowNote, 'id'> = {
			text,
			color: 'blue', // Default color, can be made configurable
			type: 'group',
			contained_node_ids: nodeIds,
			locked: false
		}

		return this.addNote(groupNote)
	}

	/**
	 * Check if editing is available (flowStore is properly initialized)
	 */
	isAvailable(): boolean {
		return !!this.flowStore.val.value
	}

	/**
	 * Clean up group notes using DAG path completion
	 */
	cleanupGroupNotes(flowNodes: { id: string; parentIds?: string[]; offset?: number }[]): void {
		if (!this.isAvailable()) {
			return
		}

		const allNotes = this.getNotes()
		const groupNotes = allNotes.filter((note) => note.type === 'group')
		if (groupNotes.length === 0) return

		let hasChanges = false
		const nodeSet = new Set(flowNodes.map((n) => n.id))

		// Step 1: Clean invalid nodes from existing group notes
		for (const note of groupNotes) {
			const originalIds = note.contained_node_ids || []
			const validIds = originalIds.filter((id) => nodeSet.has(id))

			if (validIds.length !== originalIds.length) {
				note.contained_node_ids = validIds
				hasChanges = true
			}
		}

		// Step 2: Complete paths for each group using the DAG algorithm
		const splitGroups: FlowNote[] = []

		for (const note of groupNotes) {
			const originalNodes = note.contained_node_ids || []
			if (originalNodes.length === 0) continue

			// Use the DAG path completion and splitting algorithm
			const completedGroups = completeAndSplitGroup(originalNodes, flowNodes)

			if (completedGroups.length <= 1) {
				// Single group or no change needed
				const completeNodes = completedGroups.length > 0 ? completedGroups[0] : []
				const sortedComplete = completeNodes.sort()
				const sortedOriginal = originalNodes.sort()

				if (
					sortedComplete.length !== sortedOriginal.length ||
					!sortedComplete.every((id, i) => id === sortedOriginal[i])
				) {
					note.contained_node_ids = completeNodes
					hasChanges = true
				}
			} else {
				// Multiple groups - split into separate notes
				hasChanges = true
				// Mark original note for removal
				note.contained_node_ids = []

				// Create new notes for each completed group
				for (const completedGroup of completedGroups) {
					splitGroups.push({
						...note,
						id: generateId(),
						contained_node_ids: completedGroup
					})
				}
			}
		}

		// Remove empty group notes and add split component notes
		const nonEmptyGroupNotes = groupNotes.filter(
			(note) => (note.contained_node_ids?.length || 0) > 0
		)

		if (hasChanges || splitGroups.length > 0) {
			const updatedNotes = [
				...allNotes.filter((note) => note.type !== 'group'),
				...nonEmptyGroupNotes,
				...splitGroups
			]
			this.setNotes(updatedNotes)
		}
	}
}

/**
 * Context type for NoteEditor
 */
export type NoteEditorContext = {
	noteEditor: NoteEditor
}

const CONTEXT_KEY = 'NoteEditorContext'

/**
 * Set the NoteEditor context (used in FlowBuilder)
 */
export function setNoteEditorContext(noteEditor: NoteEditor): void {
	setContext<NoteEditorContext>(CONTEXT_KEY, { noteEditor })
}

/**
 * Get the NoteEditor context (used in components that need editing capabilities)
 */
export function getNoteEditorContext(): NoteEditorContext | undefined {
	return getContext<NoteEditorContext | undefined>(CONTEXT_KEY)
}
