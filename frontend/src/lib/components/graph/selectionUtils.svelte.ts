import type { FlowModule } from '$lib/gen'
import type { Node } from '@xyflow/svelte'

export interface SelectionState {
	selectedId: string | undefined
	selectedIds: string[]
	selectionMode: 'normal' | 'rect-select'
}

export class SelectionManager {
	public selectedIds = $state<string[]>([])
	#selectionMode = $state<'normal' | 'rect-select'>('normal')

	constructor() {}

	selectId(id: string) {
		this.selectedIds = [id]
	}

	getSelectedId(): string | undefined {
		return this.selectedIds[0] || undefined
	}

	get mode() {
		return this.#selectionMode
	}

	set mode(mode: 'normal' | 'rect-select') {
		this.#selectionMode = mode
		if (mode === 'normal') {
			// When exiting rect mode, preserve the first item if there are selections
			const firstSelected = this.selectedIds[0]
			this.selectId(firstSelected ?? 'settings')
		}
	}

	// Get hierarchical children of a node
	getNodeChildrenIds(nodeId: string, modules: FlowModule[] | undefined, nodes: Node[]): string[] {
		const module = modules?.find((m) => m.id === nodeId)
		if (!module) return []

		const childrenIds: string[] = []

		// For hierarchical modules, find all children between start and end using proper graph traversal
		if (
			module.value.type === 'forloopflow' ||
			module.value.type === 'whileloopflow' ||
			module.value.type === 'branchall' ||
			module.value.type === 'branchone'
		) {
			const endNodeId = `${nodeId}-end`
			const endNode = nodes.find((n) => n.id === endNodeId)

			if (endNode) {
				// Traverse from end node back to start using parentIds
				const visited = new Set<string>()
				const toVisit = [endNodeId]

				while (toVisit.length > 0) {
					const currentId = toVisit.shift()!
					if (visited.has(currentId) || currentId === nodeId) continue

					visited.add(currentId)
					const currentNode = nodes.find((n) => n.id === currentId)

					if (currentNode && (currentNode as any).parentIds) {
						const parentIds = (currentNode as any).parentIds as string[]
						for (const parentId of parentIds) {
							if (parentId !== nodeId && !visited.has(parentId)) {
								childrenIds.push(parentId)
								toVisit.push(parentId)
							}
						}
					}
				}
			}
		}

		return childrenIds
	}

	// Select nodes with optional hierarchical selection
	selectNodes(nodeIds: string[], addToExisting = false, modules?: FlowModule[], nodes?: Node[]) {
		// Guard against empty nodeIds or uninitialized state
		if (!nodeIds || nodeIds.length === 0) {
			if (!addToExisting) {
				this.clearSelection()
			}
			return
		}

		const newSelection = addToExisting ? [...this.selectedIds] : []

		nodeIds.forEach((nodeId) => {
			// Only add valid node IDs that exist in the current nodes
			if (!nodes || nodes.some((node) => node.id === nodeId)) {
				if (!newSelection.includes(nodeId)) {
					newSelection.push(nodeId)
				}
				// Auto-select children for hierarchical modules
				if (modules && nodes) {
					const children = this.getNodeChildrenIds(nodeId, modules, nodes)
					children.forEach((childId) => {
						if (nodes.some((node) => node.id === childId) && !newSelection.includes(childId)) {
							newSelection.push(childId)
						}
					})
				}
			}
		})

		this.selectedIds = newSelection
	}

	// Toggle node selection
	toggleNodeSelection(nodeId: string, modules?: FlowModule[], nodes?: Node[]) {
		const newSelection = [...this.selectedIds]
		if (newSelection.includes(nodeId)) {
			// Remove node
			const index = newSelection.indexOf(nodeId)
			newSelection.splice(index, 1)
			// Also remove children
			if (modules && nodes) {
				const children = this.getNodeChildrenIds(nodeId, modules, nodes)
				children.forEach((childId) => {
					const childIndex = newSelection.indexOf(childId)
					if (childIndex > -1) {
						newSelection.splice(childIndex, 1)
					}
				})
			}
		} else {
			// Add node
			newSelection.push(nodeId)
			// Auto-select children for hierarchical modules
			if (modules && nodes) {
				const children = this.getNodeChildrenIds(nodeId, modules, nodes)
				children.forEach((childId) => {
					if (!newSelection.includes(childId)) {
						newSelection.push(childId)
					}
				})
			}
		}
		this.selectedIds = newSelection
	}

	// Clear all selections
	clearSelection() {
		this.selectedIds = ['settings']
	}

	// Check if a node is selected
	isNodeSelected(nodeId: string): boolean {
		return this.selectedIds.includes(nodeId)
	}

	// Get selected node count
	get selectedCount(): number {
		return this.selectedIds.length
	}

	// Check if multiple nodes are selected
	get hasMultipleSelection(): boolean {
		return this.selectedCount > 1
	}

	// Get all selected node IDs
	get selectedNodeIds(): string[] {
		return [...this.selectedIds]
	}

	// Handle keyboard shortcuts
	handleKeyDown(event: KeyboardEvent, nodes?: Node[]) {
		if (event.key === 'Escape') {
			if (this.#selectionMode === 'rect-select') {
				// Exit rect mode (this will preserve first item via mode setter)
				this.mode = 'normal'
			}
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
