import type { Job } from '$lib/gen'

/**
 * A successful, non-scheduled, top-level script can have neither retry attempts
 * (retries only spawn after a failure, so the first attempt would not be
 * `success`) nor schedule handlers (they only fire for schedule triggers, i.e.
 * when `schedule_path` is set). Its child-job (`parent_job = ?`) query would
 * always come back empty, so it can be skipped — this runs on every script run
 * view, so avoiding the round-trip on the common success path matters.
 */
export function canSkipRetryChainQuery(job: Job): boolean {
	return (
		job.type === 'CompletedJob' &&
		job.parent_job == null &&
		job.success === true &&
		job.schedule_path == null
	)
}
