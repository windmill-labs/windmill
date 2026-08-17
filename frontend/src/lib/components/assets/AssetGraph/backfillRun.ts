// Client-side orchestration of a partition-range backfill (enterprise): one
// deployed run of the producing script per slice, launched with an explicit
// `partition` arg (the worker only resolves a partition when the arg is
// absent, so the caller-provided value wins). Slices run sequentially —
// concurrent materializations of the same ducklake table would contend on
// the catalog commit — and a failed slice does not stop the rest: each slice
// is independent, and the missing/failed set is simply the next worklist.
//
// Pure module (no Svelte runes) so the loop is unit-testable; reactive
// progress is delivered via `onUpdate` snapshots, mirroring
// `cascadeOrchestrator.ts`.

export type BackfillSliceStatus = 'pending' | 'running' | 'success' | 'failure'

export type BackfillSliceState = {
	partition: string
	status: BackfillSliceStatus
	jobId?: string
	error?: string
}

export type BackfillRunOptions = {
	/** Partition values to materialize, in run order. */
	partitions: string[]
	/** Launch one run of the producer with the given partition arg; returns the job id. */
	launch: (partition: string) => Promise<string>
	/** Resolve once the job reaches a terminal state. */
	waitTerminal: (jobId: string) => Promise<'success' | 'failure'>
	/** Snapshot of all slice states, emitted on every transition. */
	onUpdate?: (slices: BackfillSliceState[]) => void
	/** Checked before each launch; a true stop leaves the remaining slices 'pending'. */
	isCancelled?: () => boolean
	/**
	 * Cancel a job whose launch raced the cancellation — the cancel click had
	 * no job id to act on yet, so the loop cancels it as soon as the id
	 * arrives. Must not throw (the job may already be terminal).
	 */
	cancelJob?: (jobId: string) => Promise<void>
}

export type BackfillRunResult = {
	/** True when every slice ran and succeeded. */
	ok: boolean
	/** True when the loop stopped early on `isCancelled`. */
	cancelled: boolean
	slices: BackfillSliceState[]
}

export async function runBackfill(opts: BackfillRunOptions): Promise<BackfillRunResult> {
	const { partitions, launch, waitTerminal, onUpdate, isCancelled, cancelJob } = opts
	const slices: BackfillSliceState[] = partitions.map((partition) => ({
		partition,
		status: 'pending'
	}))
	const emit = () => onUpdate?.(slices.map((s) => ({ ...s })))
	emit()
	let cancelled = false
	for (const slice of slices) {
		if (isCancelled?.()) {
			cancelled = true
			break
		}
		slice.status = 'running'
		emit()
		try {
			slice.jobId = await launch(slice.partition)
			emit()
			if (isCancelled?.() && cancelJob) {
				await cancelJob(slice.jobId)
			}
			slice.status = await waitTerminal(slice.jobId)
		} catch (e) {
			slice.status = 'failure'
			slice.error = e instanceof Error ? e.message : String(e)
		}
		emit()
	}
	return {
		ok: !cancelled && slices.every((s) => s.status === 'success'),
		cancelled,
		slices
	}
}
