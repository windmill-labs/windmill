import { ConcurrencyGroupsService, type Job } from '$lib/gen'

/** A job's concurrency key, and when it was first due if its limit held it back. */
export function useConcurrencyKeyStatus(getJob: () => Job | undefined) {
	type Status = { key: string; deferred_from?: string | null } | null
	let loaded = $state<{ jobId: string; status: Status } | undefined>(undefined)

	let id = $derived(getJob()?.id)
	// Live updates do not carry a job's due time, so a limit holding it back is only seen by
	// asking: once per phase (waiting, running, done), and every 5s while it waits.
	let phase = $derived.by(() => {
		const job = getJob()
		return job?.type === 'CompletedJob' ? 'done' : job?.['running'] ? 'running' : 'waiting'
	})

	$effect(() => {
		const jobId = id
		const waiting = phase === 'waiting'
		if (!jobId) return
		let current = true
		let interval: ReturnType<typeof setInterval> | undefined = undefined
		const load = () =>
			ConcurrencyGroupsService.getConcurrencyKeyStatus({ id: jobId })
				.then((status) => {
					if (!current) return
					loaded = { jobId, status }
					// The key is written as the job is queued: none means it has no limit.
					if (!status) clearInterval(interval)
				})
				.catch(() => {})
		load()
		if (waiting) interval = setInterval(load, 5000)
		return () => {
			current = false
			clearInterval(interval)
		}
	})

	let status = $derived(loaded?.jobId === id ? loaded?.status : undefined)

	return {
		get key() {
			return status?.key
		},
		get deferredFrom() {
			return status?.deferred_from ?? undefined
		}
	}
}
