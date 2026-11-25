<script lang="ts">
	import { CircleCheck, CircleX } from 'lucide-svelte'
	import { Badge, Button } from '$lib/components/common'
	import type { FlowModule, FlowStatusModule, Job } from '$lib/gen'
	import { displayDate, type StateStore } from '$lib/utils'
	import FlowExecutionStatus from './FlowExecutionStatus.svelte'
	import FlowApprovalWaitingRow from './FlowApprovalWaitingRow.svelte'
	import { pendingAgentApproval } from '../agentRequest'
	import StepIdBadge from '../flows/content/StepIdBadge.svelte'

	interface Props {
		job: Job
		isOwner: boolean
		suspendStatus: StateStore<Record<string, { job: Job; nb: number }>>
		/** Selects the step in the run's graph. */
		onSelectStep: (stepId: string) => void
	}

	let { job, isOwner, suspendStatus, onSelectStep }: Props = $props()

	type DoneRow = {
		module: FlowModule
		status: FlowStatusModule
		/** A rejection the flow continued past (`continue_on_disapprove_timeout`). */
		rejected: boolean
	}
	type Wait = { job: Job; module: FlowModule; within?: FlowModule }

	// The run's own approval steps it went past. A subflow, branch or loop iteration keeps the
	// approvals it went past in its own run.
	let done = $derived.by((): DoneRow[] => {
		const statuses = job.flow_status?.modules ?? []
		return (job.raw_flow?.modules ?? []).flatMap((module, i): DoneRow[] => {
			const status = statuses[i]
			if (!module.suspend || status?.type !== 'Success' || status.skipped) return []
			if ((status.approvers?.length ?? 0) === 0) return []
			return [
				{ module, status, rejected: status.approvers?.some((a) => a.approved === false) ?? false }
			]
		})
	})

	// Every approval the run waits on: its own step, or one in a subflow, branch or loop iteration
	// under it, which the run's graph shows inside the step holding it.
	let waits = $derived.by((): Wait[] => {
		const step = job.flow_status?.step ?? 0
		const own =
			job.flow_status?.modules?.[step]?.type === 'WaitingForEvents'
				? job.raw_flow?.modules?.[step - 1]
				: undefined
		if (own?.suspend) return [{ job, module: own }]
		const holding = job.raw_flow?.modules?.[step]
		return Object.values(suspendStatus.val ?? {}).flatMap(({ job: waiting }): Wait[] => {
			const module = waiting.raw_flow?.modules?.[(waiting.flow_status?.step ?? 0) - 1]
			if (waiting.id === job.id || !module?.suspend) return []
			return [{ job: waiting, module, within: holding }]
		})
	})

	// Waits the rows don't hold: an agent asking for a tool approval, or a run whose steps are not
	// loaded.
	let otherWaiting = $derived(
		!!pendingAgentApproval(job) ||
			(waits.length === 0 && Object.keys(suspendStatus.val ?? {}).length > 0)
	)

	let approvedCount = $derived(done.filter((r) => !r.rejected).length)
	let stepCount = $derived(done.length + waits.length)
</script>

{#if stepCount > 0 || otherWaiting}
	<div class="rounded-md border bg-surface-tertiary">
		<div class="flex items-center justify-between px-4 py-2 text-xs font-semibold text-emphasis">
			<span>Approvals</span>
			{#if stepCount > 0}
				<span class="font-normal text-secondary">
					{approvedCount} of {stepCount} step{stepCount === 1 ? '' : 's'} approved
				</span>
			{/if}
		</div>
		{#each done as row (row.module.id)}
			<div
				class="grid grid-cols-[16px_minmax(140px,1fr)_minmax(0,2fr)_auto] items-center gap-3 px-4 py-2 border-t text-xs"
			>
				{#if row.rejected}
					<CircleX size={14} class="text-red-500" />
				{:else}
					<CircleCheck size={14} class="text-green-500" />
				{/if}
				<!-- Into the graph below, where the step's own status, logs and result are. -->
				<span class="flex items-center gap-1 min-w-0">
					<Button
						variant="subtle"
						unifiedSize="xs"
						btnClasses="justify-start min-w-0 truncate"
						title="Show {row.module.summary || row.module.id} in the graph"
						onclick={() => onSelectStep(row.module.id)}
					>
						{row.module.summary || row.module.id}
					</Button>
					{#if row.module.summary}<StepIdBadge id={row.module.id} />{/if}
				</span>
				<span class="flex flex-wrap items-center gap-x-3 gap-y-0.5 min-w-0 text-secondary">
					<!-- Unkeyed: the status stores resume ids truncated to 16 bits, so two can collide. -->
					{#each row.status.approvers ?? [] as approver}
						<span>
							<span class="text-primary">{approver.approver}</span>
							{#if approver.approved_at}
								<span class="text-hint">· {displayDate(approver.approved_at)}</span>
							{/if}
						</span>
					{/each}
				</span>
				{#if row.rejected}
					<Badge color="red" small>Rejected, continued</Badge>
				{:else}
					<Badge color="green" small>Approved</Badge>
				{/if}
			</div>
		{/each}
		{#each waits as wait (`${wait.job.id}:${wait.module.id}`)}
			<FlowApprovalWaitingRow
				job={wait.job}
				module={wait.module}
				within={wait.within}
				{isOwner}
				onSelectStep={() => onSelectStep((wait.within ?? wait.module).id)}
			/>
		{/each}
		{#if otherWaiting}
			<div class="px-4 pt-1 pb-3 border-t">
				<FlowExecutionStatus
					{job}
					{isOwner}
					{suspendStatus}
					innerModules={job.flow_status?.modules}
					embedded
				/>
			</div>
		{/if}
	</div>
{/if}
