import type { FlowNote } from '../../gen'
import type { AssetWithAltAccessType } from '../assets/lib'
import {
	assetDisplaysAsInputInFlowGraph,
	assetDisplaysAsOutputInFlowGraph,
	NODE_WITH_READ_ASSET_Y_OFFSET,
	NODE_WITH_WRITE_ASSET_Y_OFFSET
} from './renderers/nodes/AssetNode.svelte'
import {
	AI_TOOL_BASE_OFFSET,
	AI_TOOL_ROW_OFFSET,
	BELOW_ADDITIONAL_OFFSET
} from './renderers/nodes/AIToolNode.svelte'
import { topologicalSort } from './graphBuilder.svelte'
import { GROUP_HEADER_HEIGHT } from './groupEditor.svelte'
import type { GroupDisplayState } from './groupEditor.svelte'
import type { GraphModuleState } from '.'

type NodeDep = {
	id: string
	parentIds?: string[]
	data?: { assets?: AssetWithAltAccessType[]; module?: any }
}

type ExtraSpace = { top: number; bottom: number; left: number; right: number }

const MAX_TOOLS_PER_ROW = 2

/**
 * Pre-compute extra top/bottom space each node needs for decorations
 * (assets, AI tools, group headers, group notes).
 */
export function computeNodeExtraSpace(
	graphNodes: NodeDep[],
	opts: {
		showAssets: boolean
		showNotes: boolean
		notes: FlowNote[] | undefined
		noteTextHeights: Record<string, number>
		groupDisplayState: GroupDisplayState
		insertable: boolean
		flowModuleStates: Record<string, GraphModuleState> | undefined
	}
): Map<string, ExtraSpace> | undefined {
	const extraSpace = new Map<string, ExtraSpace>()

	// 1. Assets
	if (opts.showAssets) {
		for (const node of graphNodes) {
			const assets = node.data?.assets ?? []
			if (!assets.length) continue
			const hasRead = assets.some(assetDisplaysAsInputInFlowGraph)
			const hasWrite = assets.some(assetDisplaysAsOutputInFlowGraph)
			if (hasRead || hasWrite) {
				const prev = extraSpace.get(node.id) ?? { top: 0, bottom: 0, left: 0, right: 0 }
				extraSpace.set(node.id, {
					...prev,
					top: prev.top + (hasRead ? NODE_WITH_READ_ASSET_Y_OFFSET : 0),
					bottom: prev.bottom + (hasWrite ? NODE_WITH_WRITE_ASSET_Y_OFFSET : 0)
				})
			}
		}
	}

	// 2. AI tools
	for (const node of graphNodes) {
		const mod = node.data?.module
		if (!mod || mod.value?.type !== 'aiagent') continue

		const agentActions = !opts.insertable && opts.flowModuleStates?.[node.id]?.agent_actions

		if (agentActions) {
			// Execution mode: tools below
			const totalRows = Math.ceil(agentActions.length / MAX_TOOLS_PER_ROW)
			const space = AI_TOOL_BASE_OFFSET + AI_TOOL_ROW_OFFSET * totalRows + BELOW_ADDITIONAL_OFFSET
			const prev = extraSpace.get(node.id) ?? { top: 0, bottom: 0, left: 0, right: 0 }
			extraSpace.set(node.id, { ...prev, bottom: prev.bottom + space })
		} else {
			// Edit mode: tools above
			const tools = mod.value.tools ?? []
			const totalRows = Math.ceil(tools.length / MAX_TOOLS_PER_ROW) + (opts.insertable ? 1 : 0)
			const space = AI_TOOL_BASE_OFFSET + AI_TOOL_ROW_OFFSET * totalRows
			const prev = extraSpace.get(node.id) ?? { top: 0, bottom: 0, left: 0, right: 0 }
			extraSpace.set(node.id, { ...prev, top: prev.top + space })
		}
	}

	// Topological sort (reversed: top-of-graph first) — shared by group notes and group headers
	const sortedNodes = topologicalSort(graphNodes).reverse()

	// 3. Group notes (text above topmost node in each group note)
	if (opts.showNotes) {
		const groupNotes = (opts.notes ?? []).filter((n) => n.type === 'group')
		if (groupNotes.length > 0) {
			for (const groupNote of groupNotes) {
				if (!groupNote.contained_node_ids?.length) continue
				const topmostNodeId = sortedNodes.find((node) =>
					groupNote.contained_node_ids?.includes(node.id)
				)?.id
				if (topmostNodeId) {
					const textHeight = opts.noteTextHeights[groupNote.id] || 60
					const spacing = textHeight + 16 // padding
					const prev = extraSpace.get(topmostNodeId) ?? {
						top: 0,
						bottom: 0,
						left: 0,
						right: 0
					}
					extraSpace.set(topmostNodeId, {
						...prev,
						top: Math.max(prev.top, spacing + prev.top)
					})
				}
			}
		}
	}

	// 4. Collapsed group nodes are taller than regular nodes (header + module icons)
	for (const node of graphNodes) {
		if (node.id.startsWith('collapsed-group:')) {
			const prev = extraSpace.get(node.id) ?? { top: 0, bottom: 0, left: 0, right: 0 }
			extraSpace.set(node.id, {
				...prev,
				bottom: prev.bottom + GROUP_HEADER_HEIGHT
			})
		}
	}

	// 5. Group nodes (expanded heads and collapsed) with notes need extra height
	if (opts.showNotes) {
		const noteHeights = opts.groupDisplayState.getNoteHeights()
		for (const node of graphNodes) {
			let groupId: string | undefined
			if (node.id.startsWith('group:') && !node.id.endsWith('-end')) {
				groupId = node.id.slice('group:'.length)
			} else if (node.id.startsWith('collapsed-group:')) {
				groupId = node.id.slice('collapsed-group:'.length)
			}
			if (groupId) {
				const noteHeight = noteHeights[groupId]
				if (noteHeight && noteHeight > 0) {
					const prev = extraSpace.get(node.id) ?? { top: 0, bottom: 0, left: 0, right: 0 }
					extraSpace.set(node.id, {
						...prev,
						bottom: prev.bottom + noteHeight
					})
				}
			}
		}
	}

	return extraSpace.size > 0 ? extraSpace : undefined
}
