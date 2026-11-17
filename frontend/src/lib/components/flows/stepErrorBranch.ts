import type { FlowModule } from '$lib/gen'
import { push } from '$lib/history.svelte'
import { refreshStateStore } from '$lib/svelte5Utils.svelte'
import type { FlowEditorContext } from './types'
import { getModuleArrayContainer } from './flowTree'
import { insertNewModuleAtIndex } from './flowStateUtils.svelte'

/** Whether `stepId` sits in a step list a Branch to one can be inserted into after it. */
export function canAddErrorBranch(ctx: FlowEditorContext, stepId: string): boolean {
	return getModuleArrayContainer(ctx.flowStore.val.value, stepId) != null
}

/** Turn on continue on error for `stepId` and insert a Branch to one right after it, whose one
 *  branch runs when the step failed and whose default runs otherwise. One undo entry. */
export async function addErrorBranch(
	{ flowStore, flowStateStore, history, selectionManager }: FlowEditorContext,
	stepId: string
): Promise<void> {
	const container = getModuleArrayContainer(flowStore.val.value, stepId)
	if (!container) return
	push(history, flowStore.val)
	container.modules[container.index].continue_on_error = true
	const modules = (await insertNewModuleAtIndex(
		flowStore,
		flowStateStore,
		container.modules,
		container.index + 1,
		'branchone'
	)) as FlowModule[]
	const inserted = modules[container.index + 1]
	if (inserted.value.type === 'branchone') {
		inserted.value.branches = [
			{ summary: 'On error', expr: `!!results.${stepId}?.error`, modules: [] }
		]
	}
	refreshStateStore(flowStore)
	selectionManager.selectId(inserted.id)
}
