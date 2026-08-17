import type { GroupDisplayState } from '$lib/components/graph/groupEditor.svelte'
import type {
	GroupedModulesProxy,
	PreparedStructureDelete
} from '$lib/components/graph/groupedModulesProxy.svelte'
import { findInStructure, type FlowStructureNode } from '$lib/components/graph/flowStructure'
import type { FlowModule, OpenFlow } from '$lib/gen'
import {
	collectAgentToolIds,
	collectFlowNodeIds,
	findAgentToolOwner,
	removeAgentToolOwner,
	type AgentToolOwner
} from './agentToolTree'
import type { AgentTool } from './agentToolUtils'
import { dfs } from './dfs'
import { getDependentComponents } from './flowExplorer'
import { findModuleInModules } from './flowTree'

export type DeleteSelection =
	| { kind: 'clear' }
	| {
			kind: 'select'
			id: string
	  }

type DeleteTargetBase = {
	id: string
	stateIds: string[]
}

export type PreprocessorDeleteTarget = DeleteTargetBase & {
	kind: 'preprocessor'
}

export type StructureDeleteTarget = DeleteTargetBase & {
	kind: 'structure_node'
}

export type AgentToolDeleteTarget = DeleteTargetBase & {
	kind: 'agent_tool'
}

export type DeleteTarget =
	| PreprocessorDeleteTarget
	| StructureDeleteTarget
	| AgentToolDeleteTarget

export type DeletePlan = {
	inputIds: string[]
	targets: DeleteTarget[]
	plannedStateIds: string[]
	dependents: Record<string, string[]>
	selection: DeleteSelection
	structureDelete?: PreparedStructureDelete
	removeDuplicates: boolean
}

export type ResolvedDeleteTargets = {
	targets: DeleteTarget[]
	missingIds: string[]
}

export function resolveDeleteTargets(
	tree: FlowStructureNode[],
	modules: FlowModule[],
	ids: string[],
	hasPreprocessor: boolean
): ResolvedDeleteTargets {
	const targets: DeleteTarget[] = []
	const missingIds: string[] = []
	const seenIds = new Set<string>()

	for (const id of ids) {
		if (seenIds.has(id)) {
			continue
		}
		seenIds.add(id)

		if (id === 'preprocessor') {
			if (hasPreprocessor) {
				targets.push({ kind: 'preprocessor', id, stateIds: [id] })
			} else {
				missingIds.push(id)
			}
			continue
		}

		if (findInStructure(tree, id)) {
			const module = findModuleInModules(modules, id)
			if (module) {
				targets.push({
					kind: 'structure_node',
					id,
					stateIds: collectFlowNodeIds(module)
				})
				continue
			}
		}

		const owner = findAgentToolOwner(modules, id)
		if (owner) {
			targets.push({
				kind: 'agent_tool',
				id,
				stateIds: collectAgentToolIds(owner.tool)
			})
			continue
		}

		missingIds.push(id)
	}

	return {
		targets: pruneNestedTargets(targets),
		missingIds
	}
}

export function createDeletePlan(args: {
	ids: string[]
	flow: OpenFlow
	tree: FlowStructureNode[]
	proxy: GroupedModulesProxy
	displayState: GroupDisplayState
}): DeletePlan | undefined {
	const { targets } = resolveDeleteTargets(
		args.tree,
		args.flow.value.modules,
		args.ids,
		Boolean(args.flow.value.preprocessor_module)
	)

	if (targets.length === 0) {
		return undefined
	}

	const structureIds = targets
		.filter((target): target is StructureDeleteTarget => target.kind === 'structure_node')
		.map((target) => target.id)

	const structureDelete =
		structureIds.length > 0
			? args.proxy.prepareDelete(structureIds, { displayState: args.displayState })
			: undefined

	const plannedStateIds = uniqueIds(targets.flatMap((target) => target.stateIds))

	return {
		inputIds: args.ids,
		targets,
		plannedStateIds,
		dependents: collectDeleteDependents(plannedStateIds, args.flow),
		selection: getDeleteSelection(args.ids, plannedStateIds, args.flow.value.modules),
		structureDelete,
		removeDuplicates: Boolean(structureDelete?.duplicateGroups.length)
	}
}

export function removeDeletePlanTools(
	targets: DeleteTarget[],
	modules: FlowModule[],
	onRemove?: (tool: AgentTool) => void
): string[] {
	const removedIds = new Set<string>()
	const toolTargets = targets
		.filter((target): target is AgentToolDeleteTarget => target.kind === 'agent_tool')
		.map((target) => findAgentToolOwner(modules, target.id))
		.filter((owner): owner is AgentToolOwner => Boolean(owner))
		.sort((left, right) => {
			if (left.depth !== right.depth) {
				return right.depth - left.depth
			}
			if (left.tools === right.tools) {
				return right.toolIndex - left.toolIndex
			}
			return 0
		})

	for (const owner of toolTargets) {
		const removed = removeAgentToolOwner(owner)
		if (!removed) {
			continue
		}

		onRemove?.(removed.tool)
		for (const id of removed.removedIds) {
			removedIds.add(id)
		}
	}

	return [...removedIds]
}

function pruneNestedTargets(targets: DeleteTarget[]): DeleteTarget[] {
	const descendantIds = new Set<string>()

	for (const target of targets) {
		for (const stateId of target.stateIds) {
			if (stateId !== target.id) {
				descendantIds.add(stateId)
			}
		}
	}

	return targets.filter((target) => !descendantIds.has(target.id))
}

function collectDeleteDependents(ids: string[], flow: OpenFlow): Record<string, string[]> {
	const deletingSet = new Set(ids)
	const dependents: Record<string, string[]> = {}

	for (const id of ids) {
		const dependencies = getDependentComponents(id, flow)
		for (const [dependentId, expressions] of Object.entries(dependencies)) {
			if (deletingSet.has(dependentId)) {
				continue
			}

			dependents[dependentId] = [...(dependents[dependentId] ?? []), ...expressions]
		}
	}

	return dependents
}

function getDeleteSelection(
	ids: string[],
	deletedIds: string[],
	modules: FlowModule[]
): DeleteSelection {
	if (ids.length !== 1) {
		return { kind: 'clear' }
	}

	const [id] = ids
	if (id === 'preprocessor') {
		return { kind: 'select', id: 'Input' }
	}

	const orderedIds = dfs(modules, (module) => module.id)
	const index = orderedIds.indexOf(id)
	const deletedSet = new Set(deletedIds)

	for (let i = index - 1; i >= 0; i--) {
		if (!deletedSet.has(orderedIds[i])) {
			return { kind: 'select', id: orderedIds[i] }
		}
	}
	for (let i = index + 1; i < orderedIds.length; i++) {
		if (!deletedSet.has(orderedIds[i])) {
			return { kind: 'select', id: orderedIds[i] }
		}
	}

	return { kind: 'select', id: 'settings-metadata' }
}

function uniqueIds(ids: string[]): string[] {
	return [...new Set(ids)]
}
