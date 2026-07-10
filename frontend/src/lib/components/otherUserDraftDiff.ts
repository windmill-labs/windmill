import { AppService, FlowService, ScriptService, type UserDraftItemKind } from '$lib/gen'
import type { Value } from '$lib/utils'
import { DEFAULT_DATA, extractDataConfig } from '$lib/components/raw_apps/dataTableRefUtils'

/**
 * Fetch the currently-deployed value for an item, in the same shape its draft
 * is stored — so it can be the "original" side of a draft-vs-deployed diff.
 * App drafts hold the bare `App` value (unwrap `.value`); raw-app drafts hold a
 * flat bundle (`files`/`runnables`/`data`/`summary`/`policy`/`custom_path`),
 * with `summary`/`policy`/`custom_path` living OUTSIDE `.value` on the deployed
 * row — so we project the deployed app into that bundle (via the same
 * `extractDataConfig` the editor uses) instead of diffing mismatched shapes.
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
			return (await AppService.getAppByPath({ workspace, path, getDraft: false }))
				.value as unknown as Value
		case 'raw_app': {
			const app = await AppService.getAppByPath({ workspace, path, getDraft: false })
			const v = (app.value ?? {}) as any
			return {
				files: v.files,
				runnables: v.runnables,
				data: extractDataConfig(v) ?? { ...DEFAULT_DATA },
				summary: app.summary,
				policy: app.policy,
				custom_path: app.custom_path
			} as unknown as Value
		}
		default:
			throw new Error(`Cannot diff drafts of kind ${itemKind}`)
	}
}
