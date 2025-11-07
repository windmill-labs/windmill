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
 */
export class NoteManager {
	private notes = $state<FlowNote[]>([])
	private onNotesChangeCallback?: (notes: FlowNote[]) => void
	private updateStoresCallback?: () => void

	constructor(initialNotes: FlowNote[] = []) {
		this.notes = initialNotes
	}

	/**
	 * Set the callback function to be called when notes change
	 */
	setOnNotesChangeCallback(callback: (notes: FlowNote[]) => void) {
		this.onNotesChangeCallback = callback
	}

	/**
	 * Set the callback function to be called when stores need updating
	 */
	setUpdateStoresCallback(callback: () => void) {
		this.updateStoresCallback = callback
	}

	/**
	 * Get current notes array
	 */
	getNotes(): FlowNote[] {
		return this.notes
	}

	/**
	 * Update notes array
	 */
	setNotes(newNotes: FlowNote[]) {
		this.notes = newNotes
	}

	/**
	 * Add a new note from the note tool
	 */
	addNote(newNoteFromTool: any) {
		if (!this.onNotesChangeCallback) return

		// Add the note to our separate notes array if a note was created
		if (newNoteFromTool) {
			const newNote: FlowNote = {
				id: newNoteFromTool.id,
				text: newNoteFromTool.data?.text || '',
				position: newNoteFromTool.position,
				size: { width: newNoteFromTool.width || 300, height: newNoteFromTool.height || 100 },
				color: newNoteFromTool.data?.color || 'yellow'
			}
			this.onNotesChangeCallback([...this.notes, newNote])
		}
	}

	/**
	 * Update note text
	 */
	updateText(noteId: string, text: string) {
		if (!this.onNotesChangeCallback) return
		this.onNotesChangeCallback(
			this.notes.map((note) => (note.id === noteId ? { ...note, text } : note))
		)
		this.updateStoresCallback?.()
	}

	/**
	 * Delete a note
	 */
	delete(noteId: string) {
		if (!this.onNotesChangeCallback) return
		this.onNotesChangeCallback(this.notes.filter((note) => note.id !== noteId))
		this.updateStoresCallback?.()
	}

	/**
	 * Update note position
	 */
	updatePosition(noteId: string, position: { x: number; y: number }) {
		if (!this.onNotesChangeCallback) return
		this.onNotesChangeCallback(
			this.notes.map((note) => (note.id === noteId ? { ...note, position } : note))
		)
	}

	/**
	 * Update note size
	 */
	updateSize(noteId: string, size: { width: number; height: number }) {
		if (!this.onNotesChangeCallback) return
		this.onNotesChangeCallback(
			this.notes.map((note) => (note.id === noteId ? { ...note, size } : note))
		)
	}

	/**
	 * Update note color
	 */
	updateColor(noteId: string, color: NoteColor) {
		if (!this.onNotesChangeCallback) return
		this.onNotesChangeCallback(
			this.notes.map((note) => (note.id === noteId ? { ...note, color } : note))
		)
		this.updateStoresCallback?.()
	}

	/**
	 * Update note lock state
	 */
	updateLock(noteId: string, locked: boolean) {
		if (!this.onNotesChangeCallback) return
		this.onNotesChangeCallback(
			this.notes.map((note) => (note.id === noteId ? { ...note, locked } as any : note))
		)
		this.updateStoresCallback?.()
	}

	/**
	 * Create a group note from selected node IDs
	 */
	createGroupNote(selectedNodeIds: string[]) {
		if (selectedNodeIds.length === 0 || !this.onNotesChangeCallback) return

		try {
			const groupNote = createGroupNote(selectedNodeIds)
			// For now, we need to store group notes as FlowNote format with additional properties
			// We'll add dummy position/size that will be calculated dynamically in convertNotesToNodes
			const lockedGroupNote = {
				...groupNote,
				position: { x: 0, y: 0 }, // Dummy values, will be calculated dynamically
				size: { width: 300, height: 100 }, // Dummy values, will be calculated dynamically
				locked: true,
				isGroupNote: true,
				containedNodeIds: groupNote.containedNodeIds,
				type: 'group'
			} as FlowNote & {
				locked: boolean;
				isGroupNote: boolean;
				containedNodeIds: string[];
				type: string;
			}
			this.onNotesChangeCallback([...this.notes, lockedGroupNote])
		} catch (error) {
			console.error('Failed to create group note:', error)
		}
	}

	/**
	 * Helper function to determine if a node needs additional spacing above it for group notes.
	 * Returns the height needed above the node.
	 */
	getGroupNoteHeightForNode(nodeId: string, layoutedNodes: (NodeDep & NodePos)[]): number {
		for (const note of this.notes) {
			const extendedNote = convertToExtendedNote(note as any)
			if (isGroupNote(extendedNote) && extendedNote.containedNodeIds.includes(nodeId)) {
				// Find the topmost node in this group by Y position
				const containedNodes = layoutedNodes.filter(node =>
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
	convertToNodes(currentNodes: Node[]): Node[] {
		return this.notes.map((note) => {
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
						locked: (note as any).locked || true, // Group notes are locked by default
						isGroupNote: true,
						containedNodeIds: extendedNote.containedNodeIds,
						onUpdate: (text: string) => this.updateText(extendedNote.id, text),
						onDelete: () => this.delete(extendedNote.id),
						onColorChange: (color: NoteColor) => this.updateColor(extendedNote.id, color),
						onSizeChange: (size: { width: number; height: number }) => this.updateSize(extendedNote.id, size),
						onLockToggle: (locked: boolean) => this.updateLock(extendedNote.id, locked)
					},
					style: `width: ${bounds.size.width}px; height: ${bounds.size.height}px;`,
					width: bounds.size.width,
					height: bounds.size.height,
					zIndex: -2000,
					draggable: !(note as any).locked, // Don't allow dragging locked notes
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
						onUpdate: (text: string) => this.updateText(extendedNote.id, text),
						onDelete: () => this.delete(extendedNote.id),
						onColorChange: (color: NoteColor) => this.updateColor(extendedNote.id, color),
						onSizeChange: (size: { width: number; height: number }) => this.updateSize(extendedNote.id, size),
						onLockToggle: (locked: boolean) => this.updateLock(extendedNote.id, locked)
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