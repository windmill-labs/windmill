import type { FlowRunStatus } from '../../flowRunStatus.svelte'

const NOT_RUN = '[stroke-dasharray:4_4] opacity-50'

export type EdgeRunData = {
	/** An edge into or out of an error handler that ran: always part of the path the run took. */
	handler?: true
	/** The steps at either end of the edge. */
	steps?: string[]
	/** The edge leaves a step whose failure an error handler took; `replaced` when the handler's
	 * edge leads to the same node. */
	bypassed?: true | 'replaced'
}

/** How a run draws an edge: a path the run did not take (through a step the finished run never
 * reached, or out of a step whose failure went to the handler) faded and dashed, the rest plain. */
export function edgeRunClass(status: FlowRunStatus | undefined, data: EdgeRunData): string {
	if (!status?.showsRun || data.handler) return ''
	if (data.bypassed) return data.bypassed === 'replaced' ? 'hidden' : NOT_RUN
	return data.steps?.some((id) => status.didNotRun(id)) ? NOT_RUN : ''
}
