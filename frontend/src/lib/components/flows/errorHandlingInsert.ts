import { push } from '$lib/history.svelte'
import { refreshStateStore } from '$lib/svelte5Utils.svelte'
import { sendUserToast } from '$lib/toast'
import { logFeatureUsage } from '$lib/utils/featureUsage'
import type { FlowEditorContext } from './types'
import { errorBranchesAfter, fillErrorHandler, stepList } from './errorHandling'
import { createNewModule } from './flowStateUtils.svelte'

/** Turn on continue on error for `stepId`, insert the pre-filled error handler branchone right
 *  after it, and select the branchone. One history entry. */
export async function addErrorHandling(
	{ flowStore, flowStateStore, history, selectionManager }: FlowEditorContext,
	stepId: string
): Promise<void> {
	let branchone
	try {
		branchone = await createNewModule(flowStore, flowStateStore, 'branchone')
	} catch (e) {
		sendUserToast(`Could not add error handling: ${e}`, true)
		return
	}
	// Resolved after the await, and everything below is synchronous: the flow may have been
	// replaced or edited meanwhile, and a half-applied insert must not reach history.
	const list = stepList(flowStore.val.value, stepId)
	if (!list || errorBranchesAfter(flowStore.val.value, stepId)) return
	push(history, flowStore.val)
	// Without it the step's failure stops the flow before the handler is reached.
	const key = list.module.continue_on_error ? 'insert' : 'insert_enable'
	list.module.continue_on_error = true
	fillErrorHandler(branchone, stepId)
	list.modules.splice(list.index + 1, 0, branchone)
	refreshStateStore(flowStore)
	selectionManager.selectId(branchone.id)
	logFeatureUsage('flow_editor', 'add_error_handling', { key })
}
