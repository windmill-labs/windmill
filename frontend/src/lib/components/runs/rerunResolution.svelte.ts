import { JobService } from '$lib/gen'
import { sendUserToast } from '$lib/toast'

/**
 * Carries "this run was launched to supersede failed run X" across the client-side navigation
 * from the original run page to the new one. A re-run has no back-pointer to its origin (it is
 * pushed with no `parent_job`), so the offer to resolve the original can only be made by the
 * page that launched it.
 */
type PendingRerun = { originalId: string; workspace: string }

let pending: PendingRerun | undefined = undefined

export function rememberRerunOrigin(origin: PendingRerun) {
	pending = origin
}

/**
 * Claims the pending origin if `newJobId` is the run it launched. Returns undefined otherwise,
 * so an unrelated run page never picks up a stale offer.
 */
export function claimRerunOrigin(newJobId: string): PendingRerun | undefined {
	if (!pending || pending.originalId === newJobId) return undefined
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
							// Provenance, not a person's words: sent as a closed reason so it survives
							// outside enterprise, where free-text notes are dropped.
							system_reason: 'superseded_by_rerun'
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
