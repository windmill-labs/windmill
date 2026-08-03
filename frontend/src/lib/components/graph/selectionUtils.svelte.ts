import type { Node } from '@xyflow/svelte'

/** Intent attached to a `selectId` call. `true` opens the panel even for ids that
 * would not normally trigger it; `false` marks an incidental selection (what remains
 * after a delete) and keeps it shut; omitted uses the default rules. */
export type SelectIntentOptions = {
	openPanel?: boolean
}

/** Panels reached from toolbar buttons or dedicated graph nodes rather than step
 * modules. They open on a single selection; step modules deliberately do not. */
const FLOW_LEVEL_PANEL_IDS = new Set([
	'constants',
	'failure',
	'preprocessor',
	'Input',
	'Result',
	'Trigger'
])

export function isFlowLevelPanelTarget(id: string): boolean {
	// 'settings-' prefixed, not 'settings' prefixed: step ids are user-editable, so a
	// step renamed settings_v2 must not be mistaken for the flow's settings panel.
	return id === 'settings' || id.startsWith('settings-') || FLOW_LEVEL_PANEL_IDS.has(id)
}

export class SelectionManager {
	#selectedNodes = $state<Node[] | { id: string }[]>([])
	#selectionMode = $state<'normal' | 'rect-select'>('normal')
	#clearGraphSelection: () => void = () => {}
	#onSelectIntent: ((id: string, opts?: SelectIntentOptions) => void) | undefined = undefined

	constructor() {}

	setClearGraphSelection(clearGraphSelection: () => void) {
		this.#clearGraphSelection = clearGraphSelection
	}

	/** Fires on every `selectId` call, BEFORE the same-id dedup early-return — so a
	 * consumer can react even when the id is re-selected (e.g. clicking the already
	 * selected "Settings" toolbar button to re-open a modal panel). */
	setOnSelectIntent(cb: ((id: string, opts?: SelectIntentOptions) => void) | undefined) {
		this.#onSelectIntent = cb
	}

	selectId(id: string, opts?: SelectIntentOptions) {
		this.#onSelectIntent?.(id, opts)
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

		// Before the same-id early return, like `selectId`: re-selecting an already
		// selected node must still be able to reopen its panel.
		if (nodes.length === 1) {
			this.#onSelectIntent?.(nodes[0].id)
		}

		// If the new selection is the same as the current selection, do nothing
		const newIds = nodes.map((n) => n.id).join(',')
		const currentIds = this.#selectedNodes.map((n) => n.id).join(',')
		if (newIds === currentIds) {
			return
		}

		this.#selectedNodes = nodes
	}

	// Select multiple nodes by their IDs
	selectByIds(ids: string[]) {
		if (!ids || ids.length === 0) {
			this.clearSelection()
			return
		}
		this.#selectedNodes = ids.map((id) => ({ id }))
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
