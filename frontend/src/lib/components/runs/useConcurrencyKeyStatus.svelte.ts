import { ConcurrencyGroupsService, type Job } from '$lib/gen'
import { getDbClockNow } from '$lib/forLater'

/** A job's concurrency key and, if its limit held it back, when it was first due and when it was
 * let in. */
export function useConcurrencyKeyStatus(getJob: () => Job | undefined) {
	type Status = { key: string; deferred_from?: string | null; released_at?: string | null } | null
	let loaded = $state<{ jobId: string; status: Status } | undefined>(undefined)

	let id = $derived(getJob()?.id)
	let phase = $derived.by(() => {
		const job = getJob()
		return job?.type === 'CompletedJob' ? 'done' : job?.['running'] ? 'running' : 'waiting'
	})
	let scheduledFor = $derived(getJob()?.['scheduled_for'] as string | undefined)

	// Live updates do not carry a job's due time, so a limit holding it back is only seen by
	// asking: once per phase (waiting, running, done), and every 5s while it waits once due.
	$effect(() => {
		const jobId = id
		const waiting = phase === 'waiting'
		const dueIn = scheduledFor ? new Date(scheduledFor).getTime() - getDbClockNow().getTime() : 0
		if (!jobId) return
		let current = true
		let stopped = false
		let timer: ReturnType<typeof setTimeout> | undefined = undefined
		const stop = () => {
			stopped = true
			clearTimeout(timer)
		}
		const load = () =>
			ConcurrencyGroupsService.getConcurrencyKeyStatus({ id: jobId })
				.then((status) => {
					if (!current) return
					loaded = { jobId, status }
					// The key is written as the job is queued: none means it has no limit.
					if (!status) stop()
				})
				.catch(stop)
		const poll = () => {
			if (stopped) return
			load()
			timer = setTimeout(poll, 5000)
		}
		load()
		// setTimeout fires at once past 2^31 - 1 ms.
		if (waiting) timer = setTimeout(poll, Math.min(Math.max(dueIn, 5000), 2 ** 31 - 1))
		return () => {
			current = false
			stop()
		}
	})

	let status = $derived(loaded?.jobId === id ? loaded?.status : undefined)

	return {
		get key() {
			return status?.key
		},
		get deferredFrom() {
			return status?.deferred_from ?? undefined
		},
		get releasedAt() {
			return status?.released_at ?? undefined
		}
	}
}
