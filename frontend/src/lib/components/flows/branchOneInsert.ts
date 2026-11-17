import type { FlowModule } from '$lib/gen'
import { push } from '$lib/history.svelte'
import { refreshStateStore } from '$lib/svelte5Utils.svelte'
import type { FlowEditorContext } from './types'
import { getModuleArrayContainer } from './flowTree'
import { insertNewModuleAtIndex } from './flowStateUtils.svelte'

/** Insert a Branch to one right after `stepId`, let `fill` set it up (and change the step itself),
 *  and select it. `push` comes before every change so one undo reverts all of them. */
export async function insertBranchOneAfter(
	{ flowStore, flowStateStore, history, selectionManager }: FlowEditorContext,
	stepId: string,
	fill: (inserted: FlowModule, step: FlowModule) => void
): Promise<void> {
	const container = getModuleArrayContainer(flowStore.val.value, stepId)
	if (!container) return
	push(history, flowStore.val)
	const step = container.modules[container.index]
	const modules = (await insertNewModuleAtIndex(
		flowStore,
		flowStateStore,
		container.modules,
		container.index + 1,
		'branchone'
	)) as FlowModule[]
	const inserted = modules[container.index + 1]
	fill(inserted, step)
	refreshStateStore(flowStore)
	selectionManager.selectId(inserted.id)
}
