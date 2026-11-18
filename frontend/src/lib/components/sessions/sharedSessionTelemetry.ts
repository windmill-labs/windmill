import { logFeatureUsage } from '$lib/utils/featureUsage'

/** A reader opened someone else's shared session. The batch goes out on the share's workspace
 *  route, which refuses anyone outside it, so only members are counted. */
export function logSharedSessionOpen(workspace: string) {
	logFeatureUsage('ai_session', 'shared_open', { workspace })
}

/** Where a reader took a share out of their sidebar list. */
export type SharedSessionRemoveSource = 'header' | 'sidebar'

export function logSharedSessionRemove(source: SharedSessionRemoveSource, workspace: string) {
	logFeatureUsage('ai_session', 'shared_remove', { key: source, workspace })
}
