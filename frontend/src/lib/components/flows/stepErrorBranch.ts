import type { FlowEditorContext } from './types'
import { getModuleArrayContainer } from './flowTree'
import { insertBranchOneAfter } from './branchOneInsert'

/** Whether `stepId` sits in a step list a Branch to one can be inserted into after it. */
export function canAddErrorBranch(ctx: FlowEditorContext, stepId: string): boolean {
	return getModuleArrayContainer(ctx.flowStore.val.value, stepId) != null
}

/** Turn on continue on error for `stepId` and insert a Branch to one right after it, whose one
 *  branch runs when the step failed and whose default runs otherwise. One undo entry. */
export function addErrorBranch(ctx: FlowEditorContext, stepId: string): Promise<void> {
	return insertBranchOneAfter(ctx, stepId, (inserted, step) => {
		step.continue_on_error = true
		if (inserted.value.type === 'branchone') {
			inserted.value.branches = [
				{ summary: 'On error', expr: `!!results.${stepId}?.error`, modules: [] }
			]
		}
	})
}
