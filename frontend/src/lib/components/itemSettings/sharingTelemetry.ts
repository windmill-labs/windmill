import { logFeatureUsage } from '$lib/utils/featureUsage'

/** Where the Share dialog was opened from. */
export type ShareOpenSource = 'editor_header' | 'breadcrumb' | 'list' | 'detail' | 'public_url'

export function logShareOpen(source: ShareOpenSource) {
	logFeatureUsage('sharing', 'open', { key: source })
}

/** How a first-deploy dialog ended. */
export function logFirstDeploy(outcome: 'deployed' | 'cancelled') {
	logFeatureUsage('sharing', 'first_deploy', { key: outcome })
}
