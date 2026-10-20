<script lang="ts" module>
	import type { Job } from '$lib/gen'

	/** The approval a flow's current agent step is parked on: the step stays in progress while
	 *  its own job waits. */
	export function pendingAgentApproval(job: Job | undefined) {
		const mod = job?.flow_status?.modules?.[job?.flow_status?.step]
		const actions = mod?.agent_actions
		if (mod?.type !== 'InProgress' || !mod.job || !actions?.length) return undefined
		const last = actions[actions.length - 1]
		if (last.type !== 'approval' || mod.agent_actions_success?.[actions.length - 1] !== undefined) {
			return undefined
		}
		return { agentJobId: mod.job, action: last }
	}
</script>

<script lang="ts">
	import { JobService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { Button } from './common'
	import TextInput from './text_input/TextInput.svelte'
	import { displayDate } from '$lib/utils'

	interface Props {
		/** The workspace the job ran in, never the navigation workspace. */
		workspaceId: string
		/** The agent step's own job: it is the one parked, so it is the one answered. */
		agentJobId: string
		/** Its call id goes with the answer, so a page left open on an earlier request cannot
		 *  answer the one the agent waits on now. */
		request: { call_id: string; message: string; expires_at: string }
		/** Whether the request still waits for an answer. */
		pending: boolean
	}

	let { workspaceId, agentJobId, request, pending }: Props = $props()

	let comment = $state('')
	let loading = $state(false)
	let answered = $state<boolean | undefined>(undefined)

	async function answer(approved: boolean) {
		loading = true
		try {
			await JobService.resumeSuspended({
				workspace: workspaceId,
				jobId: agentJobId,
				requestBody: {
					approved,
					payload: {
						call_id: request.call_id,
						...(comment.trim() ? { comment: comment.trim() } : {})
					}
				}
			})
			answered = approved
		} catch (e: any) {
			sendUserToast(e?.body ?? e?.message ?? 'Failed to answer the request', true)
		} finally {
			loading = false
		}
	}
</script>

<div class="flex flex-col gap-2 w-full text-xs">
	<div class="text-emphasis font-semibold">The agent asks for approval</div>
	<div class="text-primary whitespace-pre-wrap">{request.message}</div>
	{#if answered !== undefined}
		<div class="text-secondary"
			>{answered ? 'Approved' : 'Rejected'}, the agent resumes shortly.</div
		>
	{:else if pending}
		<div class="text-hint">Expires {displayDate(request.expires_at)}</div>
		<TextInput
			bind:value={comment}
			inputProps={{ placeholder: 'Comment for the agent (optional)', disabled: loading }}
			size="sm"
		/>
		<div class="flex gap-2 justify-end">
			<Button
				variant="default"
				destructive
				unifiedSize="sm"
				disabled={loading}
				onClick={() => answer(false)}
			>
				Reject
			</Button>
			<Button variant="accent" unifiedSize="sm" disabled={loading} onClick={() => answer(true)}>
				Approve
			</Button>
		</div>
	{/if}
</div>
