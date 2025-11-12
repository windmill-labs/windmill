import type { FlowNote } from '$lib/gen'
import type { Node } from '@xyflow/svelte'
import { calculateNodesBounds } from './util'
import {
	getLayoutSignature,
	getPropertySignature,
	calculateAllNoteZIndexes
} from './groupNoteUtils'
import { deepEqual } from 'fast-equals'

export type NodePosition = {
	id: string
	position: { x: number; y: number }
}

export type TextHeightCacheEntry = {
	content: string
	height: number
}

/**
 * Utility class for managing flow notes including regular and group notes
 * This is now a stateless utility that operates on passed note data
 */
export class NoteManager {
	#cache: Record<string, TextHeightCacheEntry> = $state({})
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
	#editMode: boolean = false

	// Selection state
	selectedNoteId = $state<string | undefined>(undefined)

	constructor(
		notes: () => FlowNote[],
		setNodes: (nodes: Node[]) => void,
		getNodes: () => Node[],
		editMode: boolean
	) {
		this.#notes = notes
		this.#setNodes = setNodes
		this.#getNodes = getNodes
		this.#editMode = editMode

		// Effect to monitor note changes with dual signature tracking
		$effect(() => {
			const currentNotes = this.#notes()
			const currentLayoutSignature = getLayoutSignature(currentNotes)
			const currentPropertySignature = getPropertySignature(currentNotes)

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
	}

	/**
	 * Triggers a re-render of the graph by incrementing the render count
	 */
	render(): void {
		this.renderCount++
	}

	getCache(): Record<string, TextHeightCacheEntry> {
		return this.#cache
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
			if (!note) return node

			// Clone the node to avoid mutation
			const updatedNode = { ...node, data: { ...node.data } }

			// Update properties that don't affect layout
			if (updatedNode.data) {
				updatedNode.data.text = note.text
				updatedNode.data.color = note.color
				updatedNode.data.locked = note.locked || false
			}

			// Update draggable property based on lock state and edit mode
			const isGroupNote = note.type === 'group'
			updatedNode.draggable = isGroupNote ? false : this.#editMode && !note.locked
			if (!isGroupNote) {
				updatedNode.width = note.size.width
				updatedNode.height = note.size.height
				updatedNode.position = note.position
			}

			return updatedNode
		})

		// Use setter function to trigger reactivity
		this.#setNodes(updatedNodes)
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

		const padding = 16

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
	 * Create common data object for note nodes (rendering-only version)
	 */
	private createNoteData(
		note: FlowNote,
		onTextHeightChange: (noteId: string, height: number) => void,
		isGroupNote: boolean,
		editMode: boolean
	) {
		return {
			noteId: note.id,
			text: note.text,
			color: note.color,
			locked: note.locked || false,
			isGroupNote,
			editMode,
			...(isGroupNote && { containedNodeIds: note.contained_node_ids || [] }),
			// Text height tracking for layout calculations
			onTextHeightChange: (textHeight: number) => {
				onTextHeightChange(note.id, textHeight)
				// Cache the text height for improved performance
				this.cacheTextHeight(note.id, note.text, textHeight)
			}
		}
	}

	/**
	 * Convert a single note to a SvelteFlow node
	 */
	convertNoteToNode(
		note: FlowNote,
		currentNodes: Node[],
		textHeights: Record<string, number>,
		onTextHeightChange: (noteId: string, height: number) => void,
		editMode: boolean = false,
		zIndex?: number
	): Node {
		const isGroupNote = note.type === 'group'

		// Calculate position and size based on note type
		const { position, size } = isGroupNote
			? this.calculateGroupNoteLayout(note, currentNodes, textHeights[note.id] || 60)
			: { position: note.position, size: note.size }

		return {
			id: note.id,
			type: 'note',
			position,
			data: this.createNoteData(note, onTextHeightChange, isGroupNote, editMode),
			style: `width: ${size.width}px; height: ${size.height}px;`,
			width: size.width,
			height: size.height,
			zIndex: zIndex ?? -2000, // Use provided zIndex or fallback
			draggable: isGroupNote ? false : editMode && !note.locked,
			selectable: false
		}
	}

	/**
	 * Convert notes to SvelteFlow nodes (rendering-only)
	 */
	convertToNodes(
		notes: FlowNote[],
		currentNodes: Node[],
		textHeights: Record<string, number>,
		onTextHeightChange: (noteId: string, height: number) => void,
		editMode: boolean = false,
		flowNodes?: { id: string; parentIds?: string[]; offset?: number }[]
	): Node[] {
		// Calculate z-indexes for all notes in a single traversal
		const zIndexMap = flowNodes ? calculateAllNoteZIndexes(notes, flowNodes) : {}

		return notes.map((note) =>
			this.convertNoteToNode(
				note,
				currentNodes,
				textHeights,
				onTextHeightChange,
				editMode,
				zIndexMap[note.id]
			)
		)
	}

	/**
	 * Caches text height measurements based on content hash
	 */
	cacheTextHeight(noteId: string, content: string, height: number): void {
		this.#cache[noteId] = { content, height }
		this.render()
	}

	/**
	 * Gets cached text height if content hasn't changed, otherwise returns undefined
	 */
	getCachedTextHeight(noteId: string, content: string): number | undefined {
		const cached = this.#cache[noteId]
		if (cached && cached.content === content) {
			return cached.height
		}
		return undefined
	}

	/**
	 * Gets the text height for a note, using cache if available or fallback
	 */
	getTextHeight(
		noteId: string,
		content: string,
		runtimeHeights: Record<string, number>,
		defaultHeight: number = 60
	): number {
		// Try cached height first
		const cachedHeight = this.getCachedTextHeight(noteId, content)
		if (cachedHeight !== undefined) {
			return cachedHeight
		}

		// Fall back to runtime heights
		const runtimeHeight = runtimeHeights[noteId]
		if (runtimeHeight !== undefined) {
			// Update cache for future use
			this.cacheTextHeight(noteId, content, runtimeHeight)
			return runtimeHeight
		}

		// Default fallback
		return defaultHeight
	}

	/**
	 * Clears the entire text height cache (useful for testing or when needed)
	 */
	clearTextHeightCache(): void {
		this.#cache = {}
		this.render()
	}

	/**
	 * Removes a specific note from the cache
	 */
	removeTextHeight(noteId: string): void {
		delete this.#cache[noteId]
		this.render()
	}

	/**
	 * Select a note by ID (single selection only)
	 */
	selectNote(noteId: string): void {
		this.selectedNoteId = noteId
	}

	/**
	 * Clear note selection
	 */
	clearNoteSelection(): void {
		this.selectedNoteId = undefined
	}

	/**
	 * Check if a note is currently selected
	 */
	isNoteSelected(noteId: string): boolean {
		return this.selectedNoteId === noteId
	}
}
