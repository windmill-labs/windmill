import type { GroupDisplayState } from '$lib/components/graph/groupEditor.svelte'
import type { GroupedModulesProxy } from '$lib/components/graph/groupedModulesProxy.svelte'
import type { SelectionManager } from '$lib/components/graph/selectionUtils.svelte'
import type { FlowStructureNode } from '$lib/components/graph/flowStructure'
import type { FlowModule, OpenFlow } from '$lib/gen'
import { push, type History } from '$lib/history.svelte'
import { refreshStateStore } from '$lib/svelte5Utils.svelte'
import type { StateStore } from '$lib/utils'
import { collectFlowNodeIds } from './agentToolTree'
import {
	createDeletePlan,
	removeDeletePlanTools,
	type DeletePlan,
	type DeleteTarget
} from './flowDeleteUtils'
import type { FlowState } from './flowState'
import { deleteFlowStateById } from './flowStateUtils.svelte'
import { dfsByModule } from './previousResults'

export type PreparedDeleteRequest = {
	plan: DeletePlan
	needsDependencyConfirmation: boolean
}

export type DeletePlanExecutionContext = {
	history?: History<OpenFlow>
	flowStore: StateStore<OpenFlow>
	flowStateStore: StateStore<FlowState>
	selectionManager: Pick<SelectionManager, 'clearSelection' | 'selectId'>
	proxy: GroupedModulesProxy
	displayState: GroupDisplayState
	onDelete?: (id: string) => void
}

export type DeletePlanExecutionResult = {
	removedStateIds: string[]
	removedToolStateIds: string[]
}

export function prepareDeleteRequest(args: {
	ids: string[]
	flow: OpenFlow
	tree: FlowStructureNode[]
	proxy: GroupedModulesProxy
	displayState: GroupDisplayState
}): PreparedDeleteRequest | undefined {
	const plan = createDeletePlan(args)
	if (!plan) {
		return undefined
	}

	return {
		plan,
		needsDependencyConfirmation: Object.keys(plan.dependents).length > 0
	}
}

export function executeDeletePlan(
	plan: DeletePlan,
	args: DeletePlanExecutionContext
): DeletePlanExecutionResult {
	push(args.history, args.flowStore.val)
	const hasPreprocessor = Boolean(args.flowStore.val.value.preprocessor_module)
	const removedStructureStateIds = resolveLiveStructureStateIds(
		plan.targets,
		args.flowStore.val.value.modules,
		hasPreprocessor
	)

	if (plan.selection.kind === 'clear') {
		args.selectionManager.clearSelection()
	} else {
		args.selectionManager.selectId(plan.selection.id)
	}

	if (plan.targets.some((target) => target.kind === 'preprocessor')) {
		args.flowStore.val.value.preprocessor_module = undefined
	}

	if (plan.structureIds.length > 0) {
		const structureDelete = args.proxy.prepareDelete(plan.structureIds, {
			displayState: args.displayState
		})
		structureDelete.commit({ removeDuplicates: plan.removeDuplicates })
	}

	const removedToolStateIds = removeDeletePlanTools(plan.targets, args.flowStore.val.value.modules)
	const removedStateIds = uniqueIds([...removedStructureStateIds, ...removedToolStateIds])

	for (const id of removedStateIds) {
		deleteFlowStateById(id, args.flowStateStore)
	}

	refreshStateStore(args.flowStore)

	if (plan.inputIds.length === 1) {
		args.onDelete?.(plan.targets[0]?.id ?? plan.inputIds[0])
	}

	return {
		removedStateIds,
		removedToolStateIds
	}
}

function resolveLiveStructureStateIds(
	targets: DeleteTarget[],
	modules: FlowModule[],
	hasPreprocessor: boolean
): string[] {
	const removedIds: string[] = []

	for (const target of targets) {
		if (target.kind === 'preprocessor') {
			if (hasPreprocessor) {
				removedIds.push(target.id)
			}
			continue
		}
		if (target.kind !== 'structure_node') {
			continue
		}

		const module = dfsByModule(target.id, modules)[0]
		if (module) {
			removedIds.push(...collectFlowNodeIds(module))
		}
	}

	return uniqueIds(removedIds)
}

function uniqueIds(ids: string[]): string[] {
	return [...new Set(ids)]
}
