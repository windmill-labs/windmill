import { JobService } from '$lib/gen'
import { sendUserToast } from '$lib/toast'

/**
 * Carries "this run was launched to supersede failed run X" across the client-side navigation
 * from the original run page to the new one. A re-run has no back-pointer to its origin (it is
 * pushed with no `parent_job`), so the offer to resolve the original can only be made by the
 * page that launched it.
 */
type PendingRerun = { originalId: string; rerunId: string; workspace: string }

let pending: PendingRerun | undefined = undefined

export function rememberRerunOrigin(origin: PendingRerun) {
	pending = origin
}

/**
 * Claims the pending origin only for the exact run it launched. Matching on `rerunId` rather
 * than "any job that isn't the original" is what keeps a re-run that fails, stays queued, or is
 * never opened from leaving a claim that the next successful run consumes: that run would be
 * offered as proof of a failure it has nothing to do with.
 */
export function claimRerunOrigin(newJobId: string): PendingRerun | undefined {
	if (pending?.rerunId !== newJobId) return undefined
	const claimed = pending
	pending = undefined
	return claimed
}

/** Offers to resolve the superseded failure. Never resolves without the user asking. */
export function offerToResolveOriginal(origin: PendingRerun, onResolved?: () => void) {
	// Longer than the 5s default: this asks the user to make a decision, not just informs them.
	const OFFER_DURATION_MS = 15000
	sendUserToast(
		'Re-run succeeded',
		'success',
		[
			{
				label: 'Resolve the original failure',
				callback: async () => {
					const affected = await JobService.resolveCompletedJobs({
						workspace: origin.workspace,
						requestBody: {
							job_ids: [origin.originalId],
							// Evidence, not a claim: the server proves this run superseded the failure
							// and owns the wording, which is why it survives outside enterprise where
							// free-text notes are dropped.
							superseded_by: origin.rerunId
						}
					})
					if (affected.length === 0) {
						sendUserToast('Could not resolve the original failure', true)
						return
					}
					sendUserToast('Original failure resolved')
					onResolved?.()
				}
			}
		],
		undefined,
		OFFER_DURATION_MS
	)
}
