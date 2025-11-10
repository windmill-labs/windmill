import type { FlowNote } from '$lib/gen'
import type { Node } from '@xyflow/svelte'
import type { NoteColor } from './noteColors'
import { calculateNodesBounds } from './util'

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
				color: newNoteFromTool.data?.color || 'yellow',
				type: 'free',
				locked: false
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
		return notes.map((note) => (note.id === noteId ? { ...note, locked } : note))
	}

	/**
	 * Create a group note from selected node IDs
	 */
	createGroupNote(notes: FlowNote[], selectedNodeIds: string[]): FlowNote[] {
		if (selectedNodeIds.length === 0) return notes

		const newGroupNote: FlowNote = {
			id: `group-${Date.now()}`,
			text: '',
			position: { x: 0, y: 0 }, // Will be calculated dynamically
			size: { width: 300, height: 100 }, // Will be calculated dynamically
			color: 'gray',
			type: 'group',
			locked: false,
			contained_node_ids: selectedNodeIds
		}
		return [...notes, newGroupNote]
	}

	/**
	 * Calculate position and size for group notes based on contained nodes
	 */
	calculateGroupNoteLayout(
		note: FlowNote,
		nodes: Node[],
		textHeight: number = 60
	): { position: { x: number; y: number }; size: { width: number; height: number } } {
		if (note.type !== 'group' || !note.contained_node_ids?.length) {
			return { position: note.position, size: note.size }
		}

		const containedNodes = nodes.filter((node) => note.contained_node_ids?.includes(node.id))

		if (containedNodes.length === 0) {
			return { position: note.position, size: note.size }
		}

		// Find bounds of all contained nodes
		const bounds = calculateNodesBounds(containedNodes)

		const padding = 20
		return {
			position: {
				x: bounds.minX - padding,
				y: bounds.minY - textHeight - padding
			},
			size: {
				width: bounds.maxX - bounds.minX + 2 * padding,
				height: bounds.maxY - bounds.minY + textHeight + 2 * padding
			}
		}
	}

	/**
	 * Helper function to determine if a node needs additional spacing above it for group notes.
	 * Returns the height needed above the node (text height + padding).
	 */
	getGroupNoteHeightForNode(
		notes: FlowNote[],
		nodeId: string,
		layoutedNodes: (NodeDep & NodePos)[],
		noteTextHeights: Record<string, number>
	): number {
		const PADDING = 20 // Fixed padding above and below the note text

		for (const note of notes) {
			if (note.type === 'group' && note.contained_node_ids?.includes(nodeId)) {
				// Find the topmost node in this group by Y position
				const containedNodes = layoutedNodes.filter((node) =>
					note.contained_node_ids?.includes(node.id)
				)

				if (containedNodes.length > 0) {
					const topmostNode = containedNodes.reduce((topMost, node) =>
						node.position.y < topMost.position.y ? node : topMost
					)

					// If this is the topmost node in the group, return the needed height
					if (topmostNode.id === nodeId) {
						// Use actual text height if available, otherwise default to 60
						const textHeight = noteTextHeights[note.id] || 60
						return textHeight + PADDING
					}
				}
			}
		}
		return 0
	}

	/**
	 * Create common data object for note nodes
	 */
	private createNoteData(
		note: FlowNote,
		notes: FlowNote[],
		onNotesChange: (notes: FlowNote[]) => void,
		onTextHeightChange: (noteId: string, height: number) => void,
		isGroupNote: boolean
	) {
		return {
			text: note.text,
			color: note.color,
			locked: note.locked || false,
			isGroupNote,
			...(isGroupNote && { containedNodeIds: note.contained_node_ids || [] }),
			onUpdate: (text: string) => {
				const newNotes = this.updateText(notes, note.id, text)
				onNotesChange(newNotes)
			},
			onDelete: () => {
				const newNotes = this.delete(notes, note.id)
				onNotesChange(newNotes)
			},
			onColorChange: (color: NoteColor) => {
				const newNotes = this.updateColor(notes, note.id, color)
				onNotesChange(newNotes)
			},
			onSizeChange: (size: { width: number; height: number }) => {
				const newNotes = this.updateSize(notes, note.id, size)
				onNotesChange(newNotes)
			},
			onLockToggle: (locked: boolean) => {
				const newNotes = this.updateLock(notes, note.id, locked)
				onNotesChange(newNotes)
			},
			onTextHeightChange: (textHeight: number) => {
				onTextHeightChange(note.id, textHeight)
			}
		}
	}

	/**
	 * Convert notes to SvelteFlow nodes
	 */
	convertToNodes(
		notes: FlowNote[],
		currentNodes: Node[],
		textHeights: Record<string, number>,
		onNotesChange: (notes: FlowNote[]) => void,
		onTextHeightChange: (noteId: string, height: number) => void
	): Node[] {
		return notes.map((note) => {
			const isGroupNote = note.type === 'group'

			// Calculate position and size based on note type
			const { position, size } = isGroupNote
				? this.calculateGroupNoteLayout(note, currentNodes, textHeights[note.id] || 60)
				: { position: note.position, size: note.size }

			return {
				id: note.id,
				type: 'note',
				position,
				data: this.createNoteData(note, notes, onNotesChange, onTextHeightChange, isGroupNote),
				style: `width: ${size.width}px; height: ${size.height}px;`,
				width: size.width,
				height: size.height,
				zIndex: -2000,
				draggable: isGroupNote ? false : !note.locked,
				selectable: true
			}
		})
	}
}
