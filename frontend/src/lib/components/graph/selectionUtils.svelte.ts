import type { Node } from '@xyflow/svelte'

export class SelectionManager {
	#selectedIds = $state<string[]>([])
	#selectionMode = $state<'normal' | 'rect-select'>('normal')

	constructor() {}

	selectId(id: string) {
		if (this.#selectedIds.length === 1 && this.#selectedIds[0] === id) {
			return
		}
		this.#selectedIds = [id]
	}

	getSelectedId(): string {
		return this.#selectedIds[0] || 'settings'
	}

	get mode() {
		return this.#selectionMode
	}

	set mode(mode: 'normal' | 'rect-select') {
		this.#selectionMode = mode
	}

	get selectedIds() {
		if (this.#selectedIds.length === 0) {
			return ['settings']
		}
		return [...this.#selectedIds]
	}

	// Select nodes with optional hierarchical selection
	selectNodes(nodeIds: string[], addToExisting = false) {
		// Guard against empty nodeIds or uninitialized state
		if (!nodeIds || nodeIds.length === 0) {
			if (!addToExisting) {
				this.clearSelection()
			}
			return
		}

		const newSelection = addToExisting ? [...this.#selectedIds, ...nodeIds] : nodeIds

		// If the new selection is the same as the current selection, do nothing
		if (JSON.stringify(newSelection) === JSON.stringify($state.snapshot(this.#selectedIds))) {
			return
		}

		this.#selectedIds = newSelection
	}

	// Clear all selections
	clearSelection() {
		this.#selectedIds = ['settings']
	}

	// Check if a node is selected
	isNodeSelected(nodeId: string): boolean {
		return this.#selectedIds.includes(nodeId)
	}

	// Get selected node count
	get selectedCount(): number {
		return this.#selectedIds.length
	}

	// Check if multiple nodes are selected
	get hasMultipleSelection(): boolean {
		return this.selectedCount > 1
	}

	// Get all selected node IDs
	get selectedNodeIds(): string[] {
		return [...this.#selectedIds]
	}

	// Handle keyboard shortcuts
	handleKeyDown(event: KeyboardEvent, nodes?: Node[]) {
		if (event.key === 'Escape') {
			// Escape key clears selection regardless of mode
			this.clearSelection()
		} else if ((event.ctrlKey || event.metaKey) && event.key === 'a') {
			event.preventDefault()
			// Select all visible nodes (exclude note nodes)
			if (nodes) {
				const allNodeIds = nodes.filter((node) => node.type !== 'note').map((node) => node.id)
				this.selectNodes(allNodeIds)
			}
		}
	}
}
