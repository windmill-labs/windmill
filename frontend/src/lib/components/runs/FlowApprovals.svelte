<script lang="ts">
	import { resource } from 'runed'
	import { CircleCheck, CircleX } from 'lucide-svelte'
	import { Badge, Button } from '$lib/components/common'
	import { JobService, type FlowModule, type Job } from '$lib/gen'
	import { displayDate, type StateStore } from '$lib/utils'
	import FlowExecutionStatus from './FlowExecutionStatus.svelte'
	import FlowApprovalWaitingRow from './FlowApprovalWaitingRow.svelte'
	import NestedStepPath from './NestedStepPath.svelte'
	import { pendingAgentApproval } from '../agentRequest'
	import StepIdBadge from '../flows/content/StepIdBadge.svelte'

	interface Props {
		job: Job
		isOwner: boolean
		suspendStatus: StateStore<Record<string, { job: Job; nb: number; prefix?: string }>>
		/** Selects the step in the run's graph. */
		onSelectStep: (stepId: string) => void
	}

	let { job, isOwner, suspendStatus, onSelectStep }: Props = $props()

	type Approver = { approver: string; approved_at?: string; approved?: boolean }
	type DoneRow = {
		key: string
		stepId: string
		summary?: string
		approvers: Approver[]
		/** A rejection the flow continued past (`continue_on_disapprove_timeout`). */
		rejected: boolean
		/** For a step of a run nested under this one: that run, and the steps leading to it. */
		nested?: { runId: string; path: string[] }
	}
	type Wait = { job: Job; module: FlowModule; path?: string[] }

	const isRejected = (approvers: Approver[]) => approvers.some((a) => a.approved === false)

	let ownDone = $derived.by((): DoneRow[] => {
		const statuses = job.flow_status?.modules ?? []
		return (job.raw_flow?.modules ?? []).flatMap((module, i): DoneRow[] => {
			const status = statuses[i]
			const approvers = status?.approvers ?? []
			if (!module.suspend || status?.type !== 'Success' || status.skipped) return []
			if (approvers.length === 0) return []
			const summary = module.summary || undefined
			return [
				{ key: module.id, stepId: module.id, summary, approvers, rejected: isRejected(approvers) }
			]
		})
	})

	// Every approval the run waits on: its own step, or one in a run nested under it (a subflow,
	// branch or loop run), which the run's graph shows inside the step holding it.
	let waits = $derived.by((): Wait[] => {
		const step = job.flow_status?.step ?? 0
		const own =
			job.flow_status?.modules?.[step]?.type === 'WaitingForEvents'
				? job.raw_flow?.modules?.[step - 1]
				: undefined
		if (own?.suspend) return [{ job, module: own }]
		const holding = job.raw_flow?.modules?.[step]?.id
		return Object.values(suspendStatus.val ?? {}).flatMap(({ job: waiting, prefix }): Wait[] => {
			const module = waiting.raw_flow?.modules?.[(waiting.flow_status?.step ?? 0) - 1]
			if (waiting.id === job.id || !module?.suspend) return []
			// The prefix chains subflow steps only: a branch or loop run adds none.
			const subflows = prefix?.split(':').filter(Boolean) ?? []
			const path = holding && subflows[0] !== holding ? [holding, ...subflows] : subflows
			return [{ job: waiting, module, path }]
		})
	})

	// Who approved a nested run's steps is kept on that run: read once the run has steps that
	// nest one, again as its waits move on.
	let nests = $derived(
		(job.raw_flow?.modules ?? []).some((m) =>
			['flow', 'forloopflow', 'whileloopflow', 'branchone', 'branchall'].includes(m.value.type)
		)
	)
	const nestedDone = resource(
		() => ({
			nests,
			workspace: job.workspace_id,
			jobId: job.id,
			moved: `${job.type}:${job.flow_status?.step}:${waits.map((w) => w.job.id + w.module.id)}`
		}),
		async ({ nests, workspace, jobId }) =>
			nests && workspace ? JobService.listNestedApprovals({ workspace, jobId }) : []
	)

	let done = $derived([
		...ownDone,
		...(nestedDone.current ?? []).map(
			(a): DoneRow => ({
				key: `${a.job_id}:${a.step_id}`,
				stepId: a.step_id,
				summary: a.step_summary,
				approvers: a.approvers,
				rejected: isRejected(a.approvers),
				nested: { runId: a.job_id, path: a.path }
			})
		)
	])

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
		{#each done as row (row.key)}
			<div class="flex items-start gap-3 px-4 py-2 border-t text-xs">
				{#if row.rejected}
					<CircleX size={14} class="shrink-0 mt-[3px] text-red-500" />
				{:else}
					<CircleCheck size={14} class="shrink-0 mt-[3px] text-green-500" />
				{/if}
				<div class="flex flex-1 min-w-0 flex-wrap items-center gap-x-3 gap-y-0.5">
					<span class="flex flex-wrap items-center gap-1 min-w-0 w-64 max-w-full -ml-2">
						<!-- Into the graph below, where the step's own status, logs and result are. -->
						<Button
							variant="subtle"
							unifiedSize="xs"
							btnClasses="justify-start min-w-0 gap-1"
							title="Show {row.nested
								? row.nested.path[0]
								: row.summary || row.stepId} in the graph"
							onclick={() => onSelectStep(row.nested?.path[0] ?? row.stepId)}
						>
							<span class="truncate">{row.summary || row.stepId}</span>
							{#if row.summary}<StepIdBadge id={row.stepId} />{/if}
						</Button>
						{#if row.nested}
							<NestedStepPath
								path={row.nested.path}
								runId={row.nested.runId}
								workspace={job.workspace_id}
							/>
						{/if}
					</span>
					<span
						class="flex flex-1 basis-48 flex-wrap items-center gap-x-3 gap-y-0.5 min-w-0 text-secondary"
					>
						<!-- Unkeyed: the status stores resume ids truncated to 16 bits, so two can collide. -->
						{#each row.approvers as approver}
							<span>
								<span class="text-primary">{approver.approver}</span>
								{#if approver.approved_at}
									<span class="text-hint">· {displayDate(approver.approved_at)}</span>
								{/if}
							</span>
						{/each}
					</span>
				</div>
				<span class="shrink-0">
					{#if row.rejected}
						<Badge color="red" small>Rejected, continued</Badge>
					{:else}
						<Badge color="green" small>Approved</Badge>
					{/if}
				</span>
			</div>
		{/each}
		{#each waits as wait (`${wait.job.id}:${wait.module.id}`)}
			<FlowApprovalWaitingRow
				job={wait.job}
				module={wait.module}
				path={wait.path}
				{isOwner}
				onSelectStep={() => onSelectStep(wait.path?.[0] ?? wait.module.id)}
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
