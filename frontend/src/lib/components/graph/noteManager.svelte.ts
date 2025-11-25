import type { FlowNote } from '$lib/gen'
import type { Node } from '@xyflow/svelte'
import { getLayoutSignature, getPropertySignature } from './noteUtils.svelte'
import { deepEqual } from 'fast-equals'
import { untrack } from 'svelte'

/**
 * Utility class for managing flow note text height caching, selection, and fine-grained reactivity
 * Handles both fast visual updates and structural changes
 */
export class NoteManager {
	renderCount = $state(0)

	// Track notes for layout change detection
	#notes: () => FlowNote[]
	#previousLayoutSignature: ReturnType<typeof getLayoutSignature> = $state({
		notesCount: 0,
		noteIds: [],
		groupMemberships: []
	})
	#previousPropertySignature: ReturnType<typeof getPropertySignature> = $state([])

	// Function to update nodes array with reactivity
	#setNodes: (nodes: Node[]) => void
	#getNodes: () => Node[]

	// Selection state
	#selectedNoteId = $state<string | undefined>(undefined)

	constructor(notes: () => FlowNote[], setNodes: (nodes: Node[]) => void, getNodes: () => Node[]) {
		this.#notes = notes
		this.#setNodes = setNodes
		this.#getNodes = getNodes

		// Effect to monitor note changes with dual signature tracking
		$effect(() => {
			const currentNotes = this.#notes()
			const currentLayoutSignature = getLayoutSignature(currentNotes)
			const currentPropertySignature = getPropertySignature(currentNotes)

			untrack(() => {
				const hasLayoutChanges = !deepEqual(currentLayoutSignature, this.#previousLayoutSignature)
				const hasPropertyChanges = !deepEqual(
					currentPropertySignature,
					this.#previousPropertySignature
				)

				if (hasLayoutChanges) {
					// Structural changes require full re-render
					this.#previousLayoutSignature = currentLayoutSignature
					this.#previousPropertySignature = currentPropertySignature
					this.render()
				} else if (hasPropertyChanges) {
					// Property changes can be handled with fast updates
					this.#updateNodesProperties(currentNotes)
					this.#previousPropertySignature = currentPropertySignature
				}
			})
		})
	}

	/**
	 * Triggers a re-render of the graph by incrementing the render count
	 */
	render(): void {
		this.renderCount++
	}

	/**
	 * Update node properties using setter function for proper reactivity
	 * Only updates visual properties that don't affect layout
	 */
	#updateNodesProperties(currentNotes: FlowNote[]): void {
		const currentNodes = this.#getNodes()
		if (currentNodes.length === 0) return

		// Create a new array with updated nodes to trigger reactivity
		const updatedNodes = currentNodes.map((node) => {
			const note = currentNotes.find((n) => n.id === node.id)
			if (!note || node.type !== 'note') return node

			// Clone the node to avoid mutation
			const updatedNode = { ...node, data: { ...node.data } }

			// Update properties that don't affect layout
			if (updatedNode.data) {
				updatedNode.data.text = note.text
				updatedNode.data.color = note.color
				updatedNode.data.locked = note.locked || false
			}

			// Update draggable property based on lock state
			const isGroupNote = note.type === 'group'
			updatedNode.draggable = isGroupNote ? false : !note.locked

			// Update free note size and position (group notes are calculated differently)
			if (!isGroupNote && note.size && note.position) {
				updatedNode.width = note.size.width
				updatedNode.height = note.size.height
				updatedNode.position = { ...note.position }
			}

			return updatedNode
		})

		// Use setter function to trigger reactivity
		this.#setNodes(updatedNodes)
	}

	/**
	 * Select a note by ID (single selection only)
	 */
	selectNote(noteId: string): void {
		if (this.#selectedNoteId === noteId) {
			return
		}
		this.#selectedNoteId = noteId
	}

	/**
	 * Clear note selection
	 */
	clearNoteSelection(): void {
		this.#selectedNoteId = undefined
	}

	/**
	 * Deselect a note by ID (single selection only)
	 */
	deselectNote(noteId?: string): void {
		if (this.#selectedNoteId === noteId) {
			this.#selectedNoteId = undefined
		}
	}

	/**
	 * Check if a note is currently selected
	 */
	isNoteSelected(noteId: string): boolean {
		return this.#selectedNoteId === noteId
	}

	// Handle keyboard shortcuts
	handleKeyDown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			// Escape key clears selection regardless of mode
			this.clearNoteSelection()
		}
	}
}
