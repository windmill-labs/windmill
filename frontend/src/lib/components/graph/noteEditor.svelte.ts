import type { FlowNote } from '$lib/gen'
import type { StateStore } from '$lib/utils'
import type { ExtendedOpenFlow } from '../flows/types'
import type { NoteColor } from './noteColors'
import { DEFAULT_GROUP_NOTE_COLOR, getNextAvailableColor } from './noteColors'
import { generateId } from './util'
import { getContext, setContext } from 'svelte'
import { completeAndSplitGroup } from './groupDetectionUtils'

/**
 * Utility class for editing flow notes via direct flowStore mutations
 * This class is designed to be used in editor contexts via Svelte context
 */
export class NoteEditor {
	private flowStore: StateStore<ExtendedOpenFlow>
	private onNoteAdded?: () => void

	constructor(flowStore: StateStore<ExtendedOpenFlow>, onNoteAdded?: () => void) {
		this.flowStore = flowStore
		this.onNoteAdded = onNoteAdded
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

		// Call callback to enable notes display when a note is created
		this.onNoteAdded?.()

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
	 * Find which nodes from the given list are already in existing group notes
	 */
	private findNodesInExistingGroups(nodeIds: string[]): {
		overlappingGroups: FlowNote[]
		nodesInGroups: Set<string>
	} {
		const notes = this.getNotes()
		const groupNotes = notes.filter((note) => note.type === 'group')
		const overlappingGroups: FlowNote[] = []
		const nodesInGroups = new Set<string>()

		for (const groupNote of groupNotes) {
			const containedNodeIds = groupNote.contained_node_ids || []
			const hasOverlap = nodeIds.some((nodeId) => containedNodeIds.includes(nodeId))

			if (hasOverlap) {
				overlappingGroups.push(groupNote)
				containedNodeIds.forEach((nodeId) => nodesInGroups.add(nodeId))
			}
		}

		return { overlappingGroups, nodesInGroups }
	}

	/**
	 * Get smart color for group note based on existing groups
	 */
	private getSmartGroupNoteColor(nodeIds: string[]): NoteColor {
		const { overlappingGroups } = this.findNodesInExistingGroups(nodeIds)

		// If no overlapping groups, use default color
		if (overlappingGroups.length === 0) {
			return DEFAULT_GROUP_NOTE_COLOR
		}

		// Get colors used by overlapping groups
		const usedColors = new Set<NoteColor>()
		overlappingGroups.forEach((group) => {
			if (group.color) {
				usedColors.add(group.color as NoteColor)
			}
		})

		// Return next available color
		return getNextAvailableColor(usedColors)
	}

	/**
	 * Create a group note containing the specified node IDs
	 */
	createGroupNote(
		nodeIds: string[],
		text: string = '### Group note\nDouble click to edit me'
	): string {
		// Filter ids in case they contain subflow nodes
		let filteredNodeIds: string[] = nodeIds
		let subflowIds: string[] = []
		for (const id of nodeIds) {
			if (id.startsWith('subflow:')) {
				const match = id.match(/^subflow:([^:]+)/)
				if (match) {
					subflowIds.push(match[1])
				}
			}
		}
		if (subflowIds.length > 0) {
			filteredNodeIds = filteredNodeIds.filter((id) => !subflowIds.includes(id))
			filteredNodeIds = [...filteredNodeIds, ...subflowIds]
		}

		// Position and size will be calculated dynamically by layout
		const smartColor = this.getSmartGroupNoteColor(filteredNodeIds)

		const groupNote: Omit<FlowNote, 'id'> = {
			text,
			color: smartColor,
			type: 'group',
			contained_node_ids: filteredNodeIds,
			locked: false
		}

		return this.addNote(groupNote)
	}

	/**
	 * Check if a node is the only member of an existing group note
	 */
	isNodeOnlyMemberOfGroupNote(nodeId: string): boolean {
		const notes = this.getNotes()
		const groupNotes = notes.filter((note) => note.type === 'group')

		for (const groupNote of groupNotes) {
			const containedNodeIds = groupNote.contained_node_ids || []
			if (containedNodeIds.length === 1 && containedNodeIds.includes(nodeId)) {
				return true
			}
		}

		return false
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
