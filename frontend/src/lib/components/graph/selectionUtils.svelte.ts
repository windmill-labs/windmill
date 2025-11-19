import type { Node } from '@xyflow/svelte'

export class SelectionManager {
	#selectedNodes = $state<Node[] | { id: string }[]>([])
	#selectionMode = $state<'normal' | 'rect-select'>('normal')
	#clearGraphSelection: () => void = () => {}

	constructor() {}

	setClearGraphSelection(clearGraphSelection: () => void) {
		this.#clearGraphSelection = clearGraphSelection
	}

	selectId(id: string) {
		if (this.#selectedNodes.length === 1 && this.#selectedNodes[0].id === id) {
			return
		}
		this.#clearGraphSelection()
		this.#selectedNodes = [{ id }]
	}

	getSelectedId(): string {
		if (this.#selectedNodes.length === 0) {
			return 'settings'
		}
		const selectedNode = this.#selectedNodes[0]

		if (selectedNode['type'] === 'branchOneEnd') {
			const id = selectedNode.id.replace(/-end$/, '')
			if (id !== '') {
				return id
			}
		} else if (selectedNode['type'] === 'branchAllEnd') {
			const id = selectedNode.id.replace(/-end$/, '')
			if (id !== '') {
				return id
			}
		} else if (selectedNode['type'] === 'forLoopStart') {
			const id = selectedNode.id.replace(/-start$/, '')
			if (id !== '') {
				return id
			}
		} else if (selectedNode['type'] === 'forLoopEnd') {
			const id = selectedNode.id.replace(/-end$/, '')
			if (id !== '') {
				return id
			}
		} else if (selectedNode['type'] === 'subflowBound') {
			const id = selectedNode.id.replace(/-subflow-end$/, '')
			if (id !== '') {
				return id
			}
		}
		return selectedNode.id
	}

	get mode() {
		return this.#selectionMode
	}

	set mode(mode: 'normal' | 'rect-select') {
		this.#selectionMode = mode
	}

	get selectedIds() {
		if (this.#selectedNodes.length === 0) {
			return ['settings']
		}
		return [...this.#selectedNodes.map((node) => node.id)]
	}

	// Select nodes with optional hierarchical selection
	selectNodes(nodes: Node[]) {
		// Guard against empty nodeIds or uninitialized state
		if (!nodes || nodes.length === 0) {
			this.clearSelection()
			return
		}

		// If the new selection is the same as the current selection, do nothing
		if (JSON.stringify(nodes) === JSON.stringify($state.snapshot(this.#selectedNodes))) {
			return
		}

		this.#selectedNodes = nodes
	}

	// Clear all selections
	clearSelection() {
		this.#selectedNodes = [{ id: 'settings' }]
	}

	// Check if a node is selected
	isNodeSelected(nodeId: string): boolean {
		return this.#selectedNodes.some((node) => node.id === nodeId)
	}

	// Handle keyboard shortcuts
	handleKeyDown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			// Escape key clears selection regardless of mode
			this.clearSelection()
			this.#clearGraphSelection()
		}
	}
}
