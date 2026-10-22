import { JobService, type FlowStatusModule, type Job } from '$lib/gen'

export type AgentRequest = Extract<
	NonNullable<FlowStatusModule['agent_actions']>[number],
	{ type: 'approval' | 'question' }
>

/** The approval or question a flow's current agent step is parked on: the step stays in
 *  progress while its own job waits. */
export function pendingAgentApproval(job: Job | undefined) {
	const mod = job?.flow_status?.modules?.[job?.flow_status?.step]
	const actions = mod?.agent_actions
	if (mod?.type !== 'InProgress' || !mod.job || !actions?.length) return undefined
	const last = actions[actions.length - 1]
	if (
		(last.type !== 'approval' && last.type !== 'question') ||
		mod.agent_actions_success?.[actions.length - 1] !== undefined
	) {
		return undefined
	}
	return { agentJobId: mod.job, action: last }
}

export type PendingAgentRequest = NonNullable<ReturnType<typeof pendingAgentApproval>>

/** `text` is the question's answer, or the reason given with a rejection. Its call id goes with
 *  the answer, so a page left open on an earlier request cannot answer the one waiting now. */
export async function answerAgentRequest(
	workspace: string,
	{ agentJobId, action }: PendingAgentRequest,
	approved: boolean,
	text?: string
) {
	await JobService.resumeSuspended({
		workspace,
		jobId: agentJobId,
		requestBody: {
			approved,
			payload: {
				call_id: action.call_id,
				...(text ? { [action.type === 'question' ? 'answer' : 'comment']: text } : {})
			}
		}
	})
}
