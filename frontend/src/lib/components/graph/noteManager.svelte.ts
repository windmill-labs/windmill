import type { FlowNote } from '$lib/gen'
import type { Node } from '@xyflow/svelte'
import type { NoteColor } from './noteColors'
import {
	createGroupNote,
	isGroupNote,
	calculateGroupNoteBounds,
	convertToExtendedNote
} from './groupNoteUtils'

type NodeDep = { id: string; parentIds?: string[]; offset?: number }
type NodePos = { position: { x: number; y: number } }

/**
 * Utility class for managing flow notes including regular and group notes
 * This is now a stateless utility that operates on passed note data
 */
export class NoteManager {
	constructor() {}

	/**
	 * Add a new note from the note tool
	 */
	addNote(notes: FlowNote[], newNoteFromTool: any): FlowNote[] {
		// Add the note to our separate notes array if a note was created
		if (newNoteFromTool) {
			const newNote: FlowNote = {
				id: newNoteFromTool.id,
				text: newNoteFromTool.data?.text || '',
				position: newNoteFromTool.position,
				size: { width: newNoteFromTool.width || 300, height: newNoteFromTool.height || 100 },
				color: newNoteFromTool.data?.color || 'yellow'
			}
			return [...notes, newNote]
		}
		return notes
	}

	/**
	 * Update note text
	 */
	updateText(notes: FlowNote[], noteId: string, text: string): FlowNote[] {
		return notes.map((note) => (note.id === noteId ? { ...note, text } : note))
	}

	/**
	 * Delete a note
	 */
	delete(notes: FlowNote[], noteId: string): FlowNote[] {
		return notes.filter((note) => note.id !== noteId)
	}

	/**
	 * Update note position
	 */
	updatePosition(
		notes: FlowNote[],
		noteId: string,
		position: { x: number; y: number }
	): FlowNote[] {
		return notes.map((note) => (note.id === noteId ? { ...note, position } : note))
	}

	/**
	 * Update note size
	 */
	updateSize(
		notes: FlowNote[],
		noteId: string,
		size: { width: number; height: number }
	): FlowNote[] {
		return notes.map((note) => (note.id === noteId ? { ...note, size } : note))
	}

	/**
	 * Update note color
	 */
	updateColor(notes: FlowNote[], noteId: string, color: NoteColor): FlowNote[] {
		return notes.map((note) => (note.id === noteId ? { ...note, color } : note))
	}

	/**
	 * Update note lock state
	 */
	updateLock(notes: FlowNote[], noteId: string, locked: boolean): FlowNote[] {
		return notes.map((note) => (note.id === noteId ? ({ ...note, locked } as any) : note))
	}

	/**
	 * Create a group note from selected node IDs
	 */
	createGroupNote(notes: FlowNote[], selectedNodeIds: string[]): FlowNote[] {
		if (selectedNodeIds.length === 0) return notes

		try {
			const groupNote = createGroupNote(selectedNodeIds)
			// For now, we need to store group notes as FlowNote format with additional properties
			// We'll add dummy position/size that will be calculated dynamically in convertNotesToNodes
			const newGroupNote = {
				...groupNote,
				position: { x: 0, y: 0 }, // Dummy values, will be calculated dynamically
				size: { width: 300, height: 100 }, // Dummy values, will be calculated dynamically
				locked: false, // Group notes are not locked, just not movable/resizable
				isGroupNote: true,
				containedNodeIds: groupNote.containedNodeIds,
				type: 'group'
			} as FlowNote & {
				locked: boolean
				isGroupNote: boolean
				containedNodeIds: string[]
				type: string
			}
			return [...notes, newGroupNote]
		} catch (error) {
			console.error('Failed to create group note:', error)
			return notes
		}
	}

	/**
	 * Helper function to determine if a node needs additional spacing above it for group notes.
	 * Returns the height needed above the node.
	 */
	getGroupNoteHeightForNode(
		notes: FlowNote[],
		nodeId: string,
		layoutedNodes: (NodeDep & NodePos)[]
	): number {
		for (const note of notes) {
			const extendedNote = convertToExtendedNote(note as any)
			if (isGroupNote(extendedNote) && extendedNote.containedNodeIds.includes(nodeId)) {
				// Find the topmost node in this group by Y position
				const containedNodes = layoutedNodes.filter((node) =>
					extendedNote.containedNodeIds.includes(node.id)
				)

				if (containedNodes.length > 0) {
					const topmostNode = containedNodes.reduce((topMost, node) =>
						node.position.y < topMost.position.y ? node : topMost
					)

					// If this is the topmost node in the group, return the needed height
					if (topmostNode.id === nodeId) {
						return 60 // Height for group note text
					}
				}
			}
		}
		return 0
	}

	/**
	 * Convert notes to SvelteFlow nodes
	 */
	convertToNodes(
		notes: FlowNote[],
		currentNodes: Node[],
		onNotesChange: (notes: FlowNote[]) => void
	): Node[] {
		return notes.map((note) => {
			const extendedNote = convertToExtendedNote(note as any)

			if (isGroupNote(extendedNote)) {
				// Calculate dynamic bounds for group notes
				const bounds = calculateGroupNoteBounds(extendedNote, currentNodes, 60)

				return {
					id: extendedNote.id,
					type: 'note',
					position: bounds.position,
					data: {
						text: extendedNote.text,
						color: extendedNote.color,
						locked: false, // Group notes are not locked - they can be edited
						isGroupNote: true,
						containedNodeIds: extendedNote.containedNodeIds,
						onUpdate: (text: string) => {
							const newNotes = this.updateText(notes, extendedNote.id, text)
							onNotesChange(newNotes)
						},
						onDelete: () => {
							const newNotes = this.delete(notes, extendedNote.id)
							onNotesChange(newNotes)
						},
						onColorChange: (color: NoteColor) => {
							const newNotes = this.updateColor(notes, extendedNote.id, color)
							onNotesChange(newNotes)
						},
						onSizeChange: (size: { width: number; height: number }) => {
							const newNotes = this.updateSize(notes, extendedNote.id, size)
							onNotesChange(newNotes)
						},
						onLockToggle: (locked: boolean) => {
							const newNotes = this.updateLock(notes, extendedNote.id, locked)
							onNotesChange(newNotes)
						}
					},
					style: `width: ${bounds.size.width}px; height: ${bounds.size.height}px;`,
					width: bounds.size.width,
					height: bounds.size.height,
					zIndex: -2000,
					draggable: false, // Group notes cannot be moved - position is determined by contained nodes
					selectable: true
				}
			} else {
				// Handle regular notes
				return {
					id: extendedNote.id,
					type: 'note',
					position: extendedNote.position,
					data: {
						text: extendedNote.text,
						color: extendedNote.color,
						locked: (note as any).locked || false,
						isGroupNote: false,
						onUpdate: (text: string) => {
							const newNotes = this.updateText(notes, extendedNote.id, text)
							onNotesChange(newNotes)
						},
						onDelete: () => {
							const newNotes = this.delete(notes, extendedNote.id)
							onNotesChange(newNotes)
						},
						onColorChange: (color: NoteColor) => {
							const newNotes = this.updateColor(notes, extendedNote.id, color)
							onNotesChange(newNotes)
						},
						onSizeChange: (size: { width: number; height: number }) => {
							const newNotes = this.updateSize(notes, extendedNote.id, size)
							onNotesChange(newNotes)
						},
						onLockToggle: (locked: boolean) => {
							const newNotes = this.updateLock(notes, extendedNote.id, locked)
							onNotesChange(newNotes)
						}
					},
					style: `width: ${extendedNote.size.width}px; height: ${extendedNote.size.height}px;`,
					width: extendedNote.size.width,
					height: extendedNote.size.height,
					zIndex: -2000,
					draggable: !(note as any).locked, // Don't allow dragging locked notes
					selectable: true
				}
			}
		})
	}
}
