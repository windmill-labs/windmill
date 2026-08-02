import { sendUserToast } from '$lib/toast'
import { queuedWithoutWorkerMessage } from './missingWorker'

/**
 * Poll options for a job that writes (row edits, DDL, arbitrary SQL). Such a job
 * is never abandoned (see `sideEffecting` in `pollJobResult`), so this is what
 * explains the wait when it sits on a tag no worker serves.
 */
export const writingJobOptions = {
	sideEffecting: true,
	onNoWorkerForTag: (tag: string) => sendUserToast(queuedWithoutWorkerMessage(tag), true)
}
