import { JobService, type FlowStatus, type FlowStatusModule } from '$lib/gen'

export type AgentRequest = Extract<
	NonNullable<FlowStatusModule['agent_actions']>[number],
	{ type: 'approval' | 'question' }
>

/** The approval or question an agent step's actions end on, while nobody has answered it. */
export function waitingRequest(
	mod: Pick<FlowStatusModule, 'agent_actions' | 'agent_actions_success'>
) {
	const actions = mod.agent_actions
	const last = actions?.[actions.length - 1]
	if (
		!last ||
		(last.type !== 'approval' && last.type !== 'question') ||
		mod.agent_actions_success?.[actions!.length - 1] !== undefined
	) {
		return undefined
	}
	return last
}

/** The approval or question a flow's current agent step is parked on: the step stays in
 *  progress while its own job waits. */
export function pendingAgentApproval(job: { flow_status?: FlowStatus } | undefined) {
	const mod = job?.flow_status?.modules?.[job?.flow_status?.step]
	if (mod?.type !== 'InProgress' || !mod.job) return undefined
	const action = waitingRequest(mod)
	return action && { agentJobId: mod.job, action }
}

/** Suspended, as the run's badge and icon say: an approval step, or an agent waiting on a person. */
export function isSuspended(job: { suspend?: number; flow_status?: FlowStatus }) {
	return !!job.suspend || !!pendingAgentApproval(job)
}

export type PendingAgentRequest = NonNullable<ReturnType<typeof pendingAgentApproval>>

/** `answer` is the question's answer, a list when several choices were picked, or the reason given
 *  with a rejection. Its call id goes with the answer, so a page left open on an earlier request
 *  cannot answer the one waiting now. */
export async function answerAgentRequest(
	workspace: string,
	{ agentJobId, action }: PendingAgentRequest,
	approved: boolean,
	answer?: string | string[]
) {
	await JobService.resumeSuspended({
		workspace,
		jobId: agentJobId,
		requestBody: {
			approved,
			payload: {
				call_id: action.call_id,
				...(answer?.length ? { [action.type === 'question' ? 'answer' : 'comment']: answer } : {})
			}
		}
	})
}
