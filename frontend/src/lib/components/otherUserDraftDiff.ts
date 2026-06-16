import { AppService, FlowService, ScriptService, type UserDraftItemKind } from '$lib/gen'
import type { Value } from '$lib/utils'

/**
 * Fetch the currently-deployed value for an item, in the same shape its draft
 * is stored — so it can be the "original" side of a draft-vs-deployed diff.
 * App / raw-app drafts hold the bare value (no wrapper), so we unwrap `.value`.
 * Only the cross-user-visible kinds have other-user drafts to diff.
 */
export async function fetchDeployedValueForDiff(
	workspace: string,
	itemKind: UserDraftItemKind,
	path: string
): Promise<Value> {
	switch (itemKind) {
		case 'script':
			return (await ScriptService.getScriptByPath({
				workspace,
				path,
				getDraft: false
			})) as unknown as Value
		case 'flow':
			return (await FlowService.getFlowByPath({ workspace, path })) as unknown as Value
		case 'app':
		case 'raw_app':
			return (await AppService.getAppByPath({ workspace, path, getDraft: false }))
				.value as unknown as Value
		default:
			throw new Error(`Cannot diff drafts of kind ${itemKind}`)
	}
}
