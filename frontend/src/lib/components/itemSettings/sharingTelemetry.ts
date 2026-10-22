import { logFeatureUsage } from '$lib/utils/featureUsage'

/** Where the Share dialog was opened from. `deploy` is a deploy that moves the item to another
 *  owner, which opens it to confirm. */
export type ShareOpenSource = 'editor_header' | 'list' | 'detail' | 'deploy'

export function logShareOpen(source: ShareOpenSource) {
	logFeatureUsage('sharing', 'open', { key: source })
}

/** How a first-deploy dialog ended. */
export function logFirstDeploy(outcome: 'deployed' | 'cancelled') {
	logFeatureUsage('sharing', 'first_deploy', { key: outcome })
}
