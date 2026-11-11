import { logFeatureUsage } from '$lib/utils/featureUsage'

/** Where the Share dialog was opened from. `deploy` is a deploy that moves the item to another
 *  owner, which opens it to confirm; `home` is a card in Home's favorites and recent items;
 *  `artifact` is an AI artifact's Share button, in the chat or on the artifacts page. */
export type ShareOpenSource = 'editor_header' | 'list' | 'detail' | 'deploy' | 'home' | 'artifact'

export function logShareOpen(source: ShareOpenSource) {
	logFeatureUsage('sharing', 'open', { key: source })
}

/** How a first-deploy dialog ended. */
export function logFirstDeploy(outcome: 'deployed' | 'cancelled') {
	logFeatureUsage('sharing', 'first_deploy', { key: outcome })
}
