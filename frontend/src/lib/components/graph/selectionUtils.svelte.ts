import type { Node } from '@xyflow/svelte'

const MULTIPLE_SELECTION_ID = 'multiple-selection'
const SETTINGS_ID = 'settings'

/**
 *
 * @param node - The node to get the module id from
 * @returns The module id from the node
 */
function getModuleIdFromNode(node: Node): string | undefined {
	if (node.type === 'branchOneEnd') {
		return /^(.*)-end$/.exec(node.id)?.[1]
	} else if (node.type === 'branchAllEnd') {
		return /^(.*)-end$/.exec(node.id)?.[1]
	} else if (node.type === 'forLoopEnd') {
		return /^(.*)-end$/.exec(node.id)?.[1]
	} else if (node.type === 'whileLoopEnd') {
		return /^(.*)-end$/.exec(node.id)?.[1]
	} else if (node.type === 'subflowBound') {
		return /^(.*)-end$/.exec(node.id)?.[1]
	}
	return undefined
}

export class SelectionManager {
	#selectionMode = $state<'normal' | 'rect-select'>('normal')
	#selectedNodesInGraph = $state<Node[]>([])
	#manualSelectedId = $state<string | undefined>(undefined)

	constructor() {}

	selectId(id: string) {
		// If not in the graph, set the selected id outside the graph
		this.#manualSelectedId = id
	}

	getSelectedId(): string {
		if (this.#manualSelectedId !== undefined) {
			return this.#manualSelectedId
		}
		if (this.#selectedNodesInGraph.length === 1) {
			const selectedNode = this.#selectedNodesInGraph[0]
			const moduleId = getModuleIdFromNode(selectedNode)
			if (moduleId) {
				return moduleId
			}
			return selectedNode.id
		} else if (this.#selectedNodesInGraph.length > 1) {
			return MULTIPLE_SELECTION_ID
		} else {
			return SETTINGS_ID
		}
	}

	get selectedNodesInGraph() {
		return this.#selectedNodesInGraph
	}

	get manualSelectedId() {
		return this.#manualSelectedId
	}

	set selectedNodesInGraph(nodes: Node[]) {
		this.#manualSelectedId = undefined
		this.#selectedNodesInGraph = nodes
	}

	get mode() {
		return this.#selectionMode
	}

	set mode(mode: 'normal' | 'rect-select') {
		this.#selectionMode = mode
	}

	// Clear all selections
	clearSelection() {
		this.#manualSelectedId = undefined
	}

	// Handle keyboard shortcuts
	handleKeyDown(event: KeyboardEvent, nodes?: Node[]) {
		if (event.key === 'Escape') {
			// Escape key clears selection regardless of mode
			this.clearSelection()
		}
	}
}
