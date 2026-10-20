<script lang="ts">
	import { JobService, type Job } from '$lib/gen'
	import AgentApprovalRequest, {
		pendingAgentApproval
	} from '$lib/components/AgentApprovalRequest.svelte'

	interface Props {
		workspace: string
		/** The flow job of the running turn: its agent step is the one parked. */
		jobId: string
	}

	let { workspace, jobId }: Props = $props()

	let job = $state<Job | undefined>(undefined)
	const pending = $derived(pendingAgentApproval(job))

	// The stream only says a call is running; the request to answer, and the job that answers
	// it, are on the flow's status.
	$effect(() => {
		const args = { workspace, id: jobId }
		let stopped = false
		const load = async () => {
			try {
				const loaded = await JobService.getJob({ ...args, noLogs: true })
				if (!stopped) job = loaded
			} catch {
				// transient; the next poll reads it again
			}
		}
		void load()
		const interval = setInterval(load, 2000)
		return () => {
			stopped = true
			clearInterval(interval)
		}
	})
</script>

{#if pending}
	<div class="rounded-md border p-3 mb-2 bg-surface-tertiary">
		{#key pending.action.call_id}
			<AgentApprovalRequest
				workspaceId={workspace}
				agentJobId={pending.agentJobId}
				request={pending.action}
				pending
			/>
		{/key}
	</div>
{/if}
