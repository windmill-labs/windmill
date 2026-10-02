import type { FlowModule } from '$lib/gen'
import { push } from '$lib/history.svelte'
import { refreshStateStore } from '$lib/svelte5Utils.svelte'
import type { FlowEditorContext } from './types'
import { addChoiceBranches } from './aiDecisionBranching'
import { getModuleArrayContainer } from './flowTree'
import { insertNewModuleAtIndex } from './flowStateUtils.svelte'

/** Insert a Branch to one right after the decision, with one branch per option of the question,
 *  and select it. */
export async function branchOnQuestion(
	{ flowStore, flowStateStore, history, selectionManager }: FlowEditorContext,
	decisionId: string,
	question: string,
	options: string[]
): Promise<void> {
	const container = getModuleArrayContainer(flowStore.val.value, decisionId)
	if (!container) return
	push(history, flowStore.val)
	const modules = (await insertNewModuleAtIndex(
		flowStore,
		flowStateStore,
		container.modules,
		container.index + 1,
		'branchone'
	)) as FlowModule[]
	const inserted = modules[container.index + 1]
	addChoiceBranches(inserted, decisionId, question, options)
	refreshStateStore(flowStore)
	selectionManager.selectId(inserted.id)
}
