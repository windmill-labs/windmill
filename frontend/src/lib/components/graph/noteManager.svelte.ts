import type { FlowNote } from '$lib/gen'
import type { Node } from '@xyflow/svelte'
import { calculateNodesBounds } from './util'

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
	#previousStructuralState: { count: number; groupMemberships: Record<string, string[]> } = $state({
		count: 0,
		groupMemberships: {}
	})
	#previousPropertyState: Record<
		string,
		{
			text: string
			locked: boolean
			color: string
			position: { x: number; y: number }
			size: { width: number; height: number }
		}
	> = $state({})

	// Function to update nodes array with reactivity
	#setNodes: (nodes: Node[]) => void
	#getNodes: () => Node[]
	#editMode: boolean = false

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

		// Effect to monitor both structural and property changes in notes
		$effect(() => {
			const currentNotes = this.#notes()
			const currentStructuralState = this.#extractStructuralState(currentNotes)
			const currentPropertyState = this.#extractPropertyState(currentNotes)

			const hasStructuralChanges = this.#hasStructuralChanges(
				currentStructuralState,
				this.#previousStructuralState
			)
			const propertyChanges = this.#getPropertyChanges(
				currentPropertyState,
				this.#previousPropertyState
			)

			if (hasStructuralChanges) {
				// Structural changes require full re-render
				this.#previousStructuralState = currentStructuralState
				this.#previousPropertyState = currentPropertyState
				this.render()
			} else if (propertyChanges.length > 0) {
				// Property changes can be handled with fast updates
				this.#updateNodesProperties(propertyChanges)
				this.#previousPropertyState = currentPropertyState
			}
		})
	}

	/**
	 * Triggers a re-render of the graph by incrementing the render count
	 */
	render(): void {
		this.renderCount++
	}

	/**
	 * Extract structural state from notes for change detection
	 */
	#extractStructuralState(notes: FlowNote[]): {
		count: number
		groupMemberships: Record<string, string[]>
	} {
		const groupMemberships: Record<string, string[]> = {}

		// Extract group memberships for group notes
		notes
			.filter((note) => note.type === 'group')
			.forEach((note) => {
				if (note.contained_node_ids) {
					groupMemberships[note.id] = [...note.contained_node_ids].sort() // Sort for consistent comparison
				}
			})

		return {
			count: notes.length,
			groupMemberships
		}
	}

	/**
	 * Check if there are structural changes that affect layout
	 */
	#hasStructuralChanges(
		current: { count: number; groupMemberships: Record<string, string[]> },
		previous: { count: number; groupMemberships: Record<string, string[]> }
	): boolean {
		// Check if note count changed
		if (current.count !== previous.count) {
			return true
		}

		// Check if group memberships changed
		const currentGroups = Object.keys(current.groupMemberships)
		const previousGroups = Object.keys(previous.groupMemberships)

		// Different number of group notes
		if (currentGroups.length !== previousGroups.length) {
			return true
		}

		// Check each group's membership
		for (const groupId of currentGroups) {
			const currentMembers = current.groupMemberships[groupId]
			const previousMembers = previous.groupMemberships[groupId]

			// Group didn't exist before or membership changed
			if (!previousMembers || !this.#arraysEqual(currentMembers, previousMembers)) {
				return true
			}
		}

		return false
	}

	/**
	 * Helper to compare two sorted arrays for equality
	 */
	#arraysEqual(a: string[], b: string[]): boolean {
		if (a.length !== b.length) {
			return false
		}
		return a.every((value, index) => value === b[index])
	}

	/**
	 * Extract property state from notes for change detection
	 */
	#extractPropertyState(
		notes: FlowNote[]
	): Record<
		string,
		{
			text: string
			locked: boolean
			color: string
			position: { x: number; y: number }
			size: { width: number; height: number }
		}
	> {
		const propertyState: Record<
			string,
			{
				text: string
				locked: boolean
				color: string
				position: { x: number; y: number }
				size: { width: number; height: number }
			}
		> = {}

		for (const note of notes) {
			propertyState[note.id] = {
				text: note.text,
				locked: note.locked || false,
				color: note.color,
				position: { ...note.position },
				size: { ...note.size }
			}
		}

		return propertyState
	}

	/**
	 * Get property changes between current and previous state
	 */
	#getPropertyChanges(
		current: Record<
			string,
			{
				text: string
				locked: boolean
				color: string
				position: { x: number; y: number }
				size: { width: number; height: number }
			}
		>,
		previous: Record<
			string,
			{
				text: string
				locked: boolean
				color: string
				position: { x: number; y: number }
				size: { width: number; height: number }
			}
		>
	): Array<{ noteId: string; property: string; oldValue: any; newValue: any }> {
		const changes: Array<{ noteId: string; property: string; oldValue: any; newValue: any }> = []

		for (const noteId of Object.keys(current)) {
			const currentNote = current[noteId]
			const previousNote = previous[noteId]

			if (!previousNote) continue // New note, will be handled by structural changes

			// Check each property for changes
			if (currentNote.text !== previousNote.text) {
				changes.push({
					noteId,
					property: 'text',
					oldValue: previousNote.text,
					newValue: currentNote.text
				})
			}
			if (currentNote.locked !== previousNote.locked) {
				changes.push({
					noteId,
					property: 'locked',
					oldValue: previousNote.locked,
					newValue: currentNote.locked
				})
			}
			if (currentNote.color !== previousNote.color) {
				changes.push({
					noteId,
					property: 'color',
					oldValue: previousNote.color,
					newValue: currentNote.color
				})
			}
			if (
				currentNote.position.x !== previousNote.position.x ||
				currentNote.position.y !== previousNote.position.y
			) {
				changes.push({
					noteId,
					property: 'position',
					oldValue: previousNote.position,
					newValue: currentNote.position
				})
			}
			if (
				currentNote.size.width !== previousNote.size.width ||
				currentNote.size.height !== previousNote.size.height
			) {
				changes.push({
					noteId,
					property: 'size',
					oldValue: previousNote.size,
					newValue: currentNote.size
				})
			}
		}

		return changes
	}

	/**
	 * Update node properties using setter function for proper reactivity
	 */
	#updateNodesProperties(
		changes: Array<{ noteId: string; property: string; oldValue: any; newValue: any }>
	): void {
		const currentNodes = this.#getNodes()
		if (currentNodes.length === 0) return

		// Create a new array with updated nodes to trigger reactivity
		const updatedNodes = currentNodes.map((node) => {
			const change = changes.find((c) => c.noteId === node.id)
			if (!change) return node

			// Clone the node to avoid mutation
			const updatedNode = { ...node, data: { ...node.data } }

			switch (change.property) {
				case 'text':
					if (updatedNode.data) updatedNode.data.text = change.newValue
					break
				case 'locked':
					if (updatedNode.data) updatedNode.data.locked = change.newValue
					// Update draggable property based on lock state and edit mode
					updatedNode.draggable = updatedNode.data.isGroupNote
						? false
						: this.#editMode && !change.newValue
					break
				case 'color':
					if (updatedNode.data) updatedNode.data.color = change.newValue
					break
				case 'position':
					updatedNode.position = { ...change.newValue }
					break
				case 'size':
					updatedNode.width = change.newValue.width
					updatedNode.height = change.newValue.height
					updatedNode.style = `width: ${change.newValue.width}px; height: ${change.newValue.height}px;`
					break
			}

			return updatedNode
		})

		// Use setter function to trigger reactivity
		this.#setNodes(updatedNodes)
	}

	getCache(): Record<string, TextHeightCacheEntry> {
		return this.#cache
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
		editMode: boolean = false
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
			zIndex: -2000,
			draggable: isGroupNote ? false : editMode && !note.locked,
			selectable: true
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
		editMode: boolean = false
	): Node[] {
		return notes.map((note) =>
			this.convertNoteToNode(note, currentNodes, textHeights, onTextHeightChange, editMode)
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
}
