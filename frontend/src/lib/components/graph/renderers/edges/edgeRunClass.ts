import type { FlowRunStatus } from '../../flowRunStatus.svelte'
import type { HandlerEdge } from '../../graphBuilder.svelte'

// `!` because xyflow styles `.svelte-flow__edge-path` with a selector of the same weight.
const TONES = {
	failure: '!stroke-red-300 dark:!stroke-red-400 ![stroke-width:1.5px]',
	recovered: '!stroke-green-400 dark:!stroke-green-500 ![stroke-width:1.5px]'
}
const NOT_RUN = '[stroke-dasharray:4_4] opacity-50'

export type EdgeRunData = {
	handler?: HandlerEdge
	/** The steps at either end of the edge. */
	steps?: string[]
	/** The edge leaves a step whose failure an error handler took; `replaced` when the handler's
	 * edge leads to the same node. */
	bypassed?: true | 'replaced'
}

/** How a run paints an edge: the error handler's path in its tone, and a path the run did not
 * take (through a step the finished run never reached, or out of a step whose failure went to the
 * handler) faded and dashed. */
export function edgeRunClass(status: FlowRunStatus | undefined, data: EdgeRunData): string {
	if (!status?.showsRun) return ''
	if (data.handler) {
		const tone = status.handlerEdgeTone(data.handler.id, data.handler.side)
		return tone ? TONES[tone] : ''
	}
	if (data.bypassed) return data.bypassed === 'replaced' ? 'hidden' : NOT_RUN
	return data.steps?.some((id) => status.didNotRun(id)) ? NOT_RUN : ''
}
