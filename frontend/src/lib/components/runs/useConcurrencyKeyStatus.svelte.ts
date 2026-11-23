import { untrack } from 'svelte'
import { ConcurrencyGroupsService, type Job } from '$lib/gen'

/** A job's concurrency key, and when it was first due if its limit held it back. */
export function useConcurrencyKeyStatus(getJob: () => Job | undefined) {
	let status: { key: string; deferred_from?: string | null } | undefined = $state(undefined)
	let lastCheck: string | undefined = undefined

	$effect(() => {
		const job = getJob()
		if (!job?.id) return
		// Each time a limit holds the job back it moves its due time, so read it again then.
		const check = `${job.id}:${job['scheduled_for'] ?? ''}`
		if (check === lastCheck) return
		const switched = !lastCheck?.startsWith(`${job.id}:`)
		lastCheck = check
		if (switched) status = undefined
		untrack(() =>
			ConcurrencyGroupsService.getConcurrencyKeyStatus({ id: job.id })
				.then((s) => {
					if (lastCheck === check) status = s ?? undefined
				})
				.catch(() => {})
		)
	})

	return {
		get key() {
			return status?.key
		},
		get deferredFrom() {
			return status?.deferred_from ?? undefined
		}
	}
}
