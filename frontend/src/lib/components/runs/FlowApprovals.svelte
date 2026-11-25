<script lang="ts">
	import { resource } from 'runed'
	import { ChevronDown, CircleCheck, CircleX, Hourglass } from 'lucide-svelte'
	import { Badge, Button } from '$lib/components/common'
	import { JobService, type FlowModule, type FlowStatusModule, type Job } from '$lib/gen'
	import { displayDate, type StateStore } from '$lib/utils'
	import FlowExecutionStatus from './FlowExecutionStatus.svelte'
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

	type Row = {
		module: FlowModule
		status: FlowStatusModule
		waiting: boolean
		/** A rejection the flow continued past (`continue_on_disapprove_timeout`). */
		rejected: boolean
	}

	// The run's own approval steps, as far as it reached them: an approved one, and the one it waits
	// on. A step inside a branch, loop or subflow belongs to that run; while it waits, the panel
	// below says so.
	let rows = $derived.by((): Row[] => {
		const modules = job.raw_flow?.modules ?? []
		const statuses = job.flow_status?.modules ?? []
		const step = job.flow_status?.step ?? 0
		const waitingIndex = statuses[step]?.type === 'WaitingForEvents' ? step - 1 : -1
		return modules.flatMap((module, i): Row[] => {
			const status = statuses[i]
			if (!module.suspend || !status) return []
			if (i === waitingIndex) return [{ module, status, waiting: true, rejected: false }]
			if (status.type === 'Success' && !status.skipped && (status.approvers?.length ?? 0) > 0)
				return [
					{
						module,
						status,
						waiting: false,
						rejected: status.approvers?.some((a) => a.approved === false) ?? false
					}
				]
			return []
		})
	})
	let waitingRow = $derived(rows.find((r) => r.waiting))
	// Waits the rows don't hold, which the panel below shows: a step inside a branch, loop or
	// subflow, or an agent asking for a tool approval.
	let otherWaiting = $derived(
		!waitingRow &&
			((!!suspendStatus.val && Object.keys(suspendStatus.val).length > 0) ||
				!!pendingAgentApproval(job))
	)

	const detail = resource(
		() => ({ workspace: job.workspace_id, jobId: job.id, stepId: waitingRow?.module.id }),
		({ workspace, jobId, stepId }) =>
			workspace && stepId
				? JobService.getApprovalStepDetail({ workspace, jobId, stepId })
				: Promise.resolve(undefined)
	)

	// Open for whoever can act on it; a long description or form takes room only for them.
	let expandedFor = $state<string | undefined>(undefined)
	let canApprove = $derived(detail.current?.can_approve ?? isOwner)
	let expanded = $derived(
		expandedFor === undefined ? canApprove : expandedFor === waitingRow?.module.id
	)

	let approvedCount = $derived(rows.filter((r) => !r.waiting && !r.rejected).length)

	// Worded as the approval page words it. Admins and flow owners can approve any step, so they
	// are not listed.
	function canApproveLabel(): string {
		const ac = detail.current?.approval_conditions
		const who = !ac?.user_auth_required
			? 'anyone with the approval link'
			: ac.user_groups_required.length > 0
				? `members of ${ac.user_groups_required.join(', ')}`
				: 'any signed-in member'
		return ac?.self_approval_disabled ? `${who} · self-approval disabled` : who
	}
</script>

{#if rows.length > 0}
	<div class="rounded-md border bg-surface-tertiary">
		<div class="flex items-center justify-between px-4 py-2 text-xs font-semibold text-emphasis">
			<span>Approvals</span>
			<span class="font-normal text-secondary">
				{approvedCount} of {rows.length} step{rows.length === 1 ? '' : 's'} approved
			</span>
		</div>
		{#each rows as row (row.module.id)}
			<div
				class="grid grid-cols-[16px_minmax(140px,1fr)_minmax(0,2fr)_auto] items-center gap-3 px-4 py-2 border-t text-xs"
			>
				{#if row.waiting}
					<Hourglass size={14} class="text-violet-500" />
				{:else if row.rejected}
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
					{#if row.waiting}
						{#if detail.current}
							<span><span class="text-hint">Can approve:</span> {canApproveLabel()}</span>
						{/if}
						{#if (detail.current?.approvals.length ?? 0) > 0}
							<span>
								{detail.current?.approvals.length} of {detail.current?.required_events} approvals
							</span>
						{/if}
					{:else}
						<!-- Unkeyed: the status stores resume ids truncated to 16 bits, so two can collide. -->
						{#each row.status.approvers ?? [] as approver}
							<span>
								<span class="text-primary">{approver.approver}</span>
								{#if approver.approved_at}
									<span class="text-hint">· {displayDate(approver.approved_at)}</span>
								{/if}
							</span>
						{/each}
					{/if}
				</span>
				{#if row.waiting}
					<span class="flex items-center gap-1">
						<Badge color="violet" small>Waiting</Badge>
						<Button
							variant="subtle"
							unifiedSize="xs"
							iconOnly
							startIcon={{ icon: ChevronDown, classes: expanded ? 'rotate-180' : '' }}
							title={expanded ? 'Hide the approval form' : 'Show the approval form'}
							aria-label={expanded ? 'Hide the approval form' : 'Show the approval form'}
							aria-expanded={expanded}
							onclick={() => (expandedFor = expanded ? '' : row.module.id)}
						/>
					</span>
				{:else if row.rejected}
					<Badge color="red" small>Rejected, continued</Badge>
				{:else}
					<Badge color="green" small>Approved</Badge>
				{/if}
			</div>
		{/each}
		{#if otherWaiting || (waitingRow && expanded)}
			<!-- The step's own approval panel, unchanged: its description, form and buttons. -->
			<div class="px-4 pt-1 pb-3">
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
{:else}
	<FlowExecutionStatus {job} {isOwner} {suspendStatus} innerModules={job.flow_status?.modules} />
{/if}
