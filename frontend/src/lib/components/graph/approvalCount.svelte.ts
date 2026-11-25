import { JobService } from '$lib/gen'
import type { GraphModuleState } from './model'

/** How many approvals a waiting step has received so far. Approvals coming in do not change the
 * run until the last one, so the count is read on its own, and only when there is more than one
 * to count. */
export function useApprovalCount(wait: () => GraphModuleState['approval_wait']) {
	let count = $state<number | undefined>(undefined)
	$effect(() => {
		const w = wait()
		if (!w || w.required <= 1 || !w.workspace) return
		const args = { workspace: w.workspace, jobId: w.job, stepId: w.step }
		const read = () =>
			JobService.getApprovalStepDetail(args)
				.then((d) => (count = d.approvals.length))
				.catch(() => {})
		read()
		const id = setInterval(read, 10000)
		return () => clearInterval(id)
	})
	return {
		/** "Waiting for approval", with "· 1 of 2" once some are in. */
		get label() {
			const w = wait()
			return `Waiting for approval${count && w ? ` · ${count} of ${w.required}` : ''}`
		}
	}
}
