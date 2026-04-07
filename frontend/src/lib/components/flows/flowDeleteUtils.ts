import type { FlowModule } from '$lib/gen'
import { findInStructure, type FlowStructureNode } from '$lib/components/graph/flowStructure'
import type { AgentTool } from './agentToolUtils'
import { removeAgentToolByIdDeep } from './agentToolUtils'

export type DeleteTargetPartition = {
	structureIds: string[]
	toolIds: string[]
}

/**
 * Split delete targets between structure-tree nodes and AI agent tool nodes.
 * AI tools are rendered in the graph but are not represented in the grouped structure tree.
 */
export function partitionDeleteTargets(
	tree: FlowStructureNode[],
	ids: string[]
): DeleteTargetPartition {
	const structureIds: string[] = []
	const toolIds: string[] = []

	for (const id of ids) {
		if (findInStructure(tree, id)) {
			structureIds.push(id)
		} else {
			toolIds.push(id)
		}
	}

	return { structureIds, toolIds }
}

/**
 * Remove AI agent tools by id and return the ids that were actually removed.
 */
export function removeToolIds(
	modules: FlowModule[],
	ids: string[],
	onRemove?: (tool: AgentTool) => void
): string[] {
	const removedIds = new Set<string>()

	for (const id of ids) {
		removeAgentToolByIdDeep(modules, id, (tool) => {
			removedIds.add(tool.id)
			onRemove?.(tool)
		})
	}

	return [...removedIds]
}
