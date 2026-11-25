<script lang="ts">
	import { resource } from 'runed'
	import { CircleCheck, CircleX, TimerOff } from 'lucide-svelte'
	import { Badge, Button } from '$lib/components/common'
	import { JobService, type FlowModule, type Job } from '$lib/gen'
	import { displayDate, type StateStore } from '$lib/utils'
	import FlowExecutionStatus from './FlowExecutionStatus.svelte'
	import FlowApprovalWaitingRow from './FlowApprovalWaitingRow.svelte'
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
	/** `chainedId`: the step's id after the subflow steps leading to it (`outer:r:s`).
	 * `graphStep`: its first id. Step ids are unique across a flow, loop and branch steps
	 * included, so that one is a node of this run's graph. */
	type Located = { chainedId: string; graphStep: string }
	type DoneRow = Located & {
		key: string
		stepId: string
		summary?: string
		approvers: Approver[]
		/** `rejected` and `timedOut`: the flow continued past it (`continue_on_disapprove_timeout`). */
		outcome: Outcome
	}
	type Wait = Located & { job: Job; module: FlowModule }

	type Outcome = 'approved' | 'rejected' | 'timedOut'
	// Entries written before `approved` was recorded are approvals.
	const outcome = (approvers: Approver[], required: number | undefined): Outcome =>
		approvers.some((a) => a.approved === false)
			? 'rejected'
			: approvers.length < (required ?? 1)
				? 'timedOut'
				: 'approved'

	let ownDone = $derived.by((): DoneRow[] => {
		const statuses = job.flow_status?.modules ?? []
		const flowStep = job.flow_status?.step ?? 0
		return (job.raw_flow?.modules ?? []).flatMap((module, i): DoneRow[] => {
			const status = statuses[i]
			const approvers = status?.approvers ?? []
			if (!module.suspend || status?.type !== 'Success' || status.skipped) return []
			// No response and the flow moved past the step after it: it timed out and continued.
			// While it waits, `step` is that next step's index.
			if (approvers.length === 0 && flowStep <= i + 1) return []
			return [
				{
					key: module.id,
					stepId: module.id,
					chainedId: module.id,
					graphStep: module.id,
					summary: module.summary || undefined,
					approvers,
					outcome: outcome(approvers, module.suspend.required_events)
				}
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
		if (own?.suspend) return [{ job, module: own, chainedId: own.id, graphStep: own.id }]
		return Object.values(suspendStatus.val ?? {}).flatMap(({ job: waiting, prefix }): Wait[] => {
			const module = waiting.raw_flow?.modules?.[(waiting.flow_status?.step ?? 0) - 1]
			if (waiting.id === job.id || !module?.suspend) return []
			// The prefix chains the subflow steps leading to the run, not loops or branches.
			const chain = [...(prefix?.split(':').filter(Boolean) ?? []), module.id]
			return [{ job: waiting, module, chainedId: chain.join(':'), graphStep: chain[0] }]
		})
	})

	// Mirrors `may_hold_approval` on the server: a subflow, or a loop or branch holding one.
	function mayHoldApproval(m: FlowModule): boolean {
		const holds = (modules: FlowModule[] | undefined) =>
			(modules ?? []).some((x) => !!x.suspend || mayHoldApproval(x))
		const v = m.value
		switch (v.type) {
			case 'flow':
				return true
			case 'forloopflow':
			case 'whileloopflow':
				return holds(v.modules)
			case 'branchone':
				return holds(v.default) || v.branches.some((b) => holds(b.modules))
			case 'branchall':
				return v.branches.some((b) => holds(b.modules))
			default:
				return false
		}
	}

	// Who approved a nested run's steps is kept on that run: read when the run has a step that
	// can hold one, then again only when its waits change or it finishes, not at every step.
	// Primitives: each poll brings a new `job` object, which would refetch on every poll.
	let nests = $derived((job.raw_flow?.modules ?? []).some(mayHoldApproval))
	let workspace = $derived(job.workspace_id)
	let jobId = $derived(job.id)
	let moved = $derived(`${job.type}:${waits.map((w) => w.job.id + w.module.id)}`)
	const nested = resource(
		() => ({ nests, workspace, jobId, moved }),
		async ({ nests, workspace, jobId }) =>
			nests && workspace ? JobService.listNestedApprovals({ workspace, jobId }) : undefined
	)

	// In the order they were decided: own and nested steps interleave in a flow's run.
	const decidedAt = (r: DoneRow) =>
		r.approvers.reduce((t, a) => (a.approved_at && a.approved_at > t ? a.approved_at : t), '')
	let done = $derived(
		[
			...ownDone,
			...(nested.current?.approvals ?? []).map(
				(a): DoneRow => ({
					key: `${a.job_id}:${a.step_id}`,
					stepId: a.step_id,
					chainedId: [...a.subflows, a.step_id].join(':'),
					graphStep: a.subflows[0] ?? a.step_id,
					summary: a.step_summary,
					approvers: a.approvers,
					outcome: outcome(a.approvers, a.required_events)
				})
			)
		].sort((a, b) => decidedAt(a).localeCompare(decidedAt(b)))
	)

	// Waits the rows don't hold: an agent asking for a tool approval, or a run whose steps are not
	// loaded.
	let otherWaiting = $derived(
		!!pendingAgentApproval(job) ||
			(waits.length === 0 && Object.keys(suspendStatus.val ?? {}).length > 0)
	)

	// Counts, not "N of M": a loop's step counts once per iteration, and the total is unknown
	// until the loop ends.
	let counts = $derived(
		[
			[done.filter((r) => r.outcome == 'approved').length, 'approved'],
			[done.filter((r) => r.outcome == 'rejected').length, 'rejected'],
			[done.filter((r) => r.outcome == 'timedOut').length, 'timed out'],
			[waits.length, 'waiting']
		]
			.filter(([n]) => n)
			.map(([n, label]) => `${n} ${label}`)
			.join(' · ')
	)
	let stepCount = $derived(done.length + waits.length)
</script>

{#if stepCount > 0 || otherWaiting}
	<div class="rounded-md border bg-surface-tertiary">
		<div class="flex items-center justify-between px-4 py-2 text-xs font-semibold text-emphasis">
			<span>Approvals</span>
			{#if stepCount > 0}
				<span class="font-normal text-secondary">{counts}</span>
			{/if}
		</div>
		{#each done as row (row.key)}
			<div class="flex items-start gap-3 px-4 py-2 border-t text-xs">
				{#if row.outcome == 'rejected'}
					<CircleX size={14} class="shrink-0 mt-[3px] text-red-500" />
				{:else if row.outcome == 'timedOut'}
					<TimerOff size={14} class="shrink-0 mt-[3px] text-yellow-500" />
				{:else}
					<CircleCheck size={14} class="shrink-0 mt-[3px] text-green-500" />
				{/if}
				<div class="flex flex-1 min-w-0 flex-wrap items-center gap-x-3 gap-y-0.5">
					<span class="flex items-center gap-1 min-w-0 w-64 max-w-full -ml-2">
						<!-- Into the graph below, where the step's own status, logs and result are. -->
						<Button
							variant="subtle"
							unifiedSize="xs"
							btnClasses="justify-start min-w-0 gap-1"
							title="Show {row.graphStep} in the graph"
							onclick={() => onSelectStep(row.graphStep)}
						>
							<span class="truncate">{row.summary || row.stepId}</span>
							{#if row.summary || row.chainedId !== row.stepId}
								<StepIdBadge id={row.chainedId} maxLength={24} />
							{/if}
						</Button>
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
					{#if row.outcome == 'rejected'}
						<Badge color="red" small>Rejected, continued</Badge>
					{:else if row.outcome == 'timedOut'}
						<Badge color="yellow" small>Timed out, continued</Badge>
					{:else}
						<Badge color="green" small>Approved</Badge>
					{/if}
				</span>
			</div>
		{/each}
		{#if nested.current?.more}
			<p class="px-4 py-2 border-t text-xs text-hint">
				More approvals in nested runs than shown here: they are on those runs' own pages.
			</p>
		{/if}
		{#each waits as wait (`${wait.job.id}:${wait.module.id}`)}
			<FlowApprovalWaitingRow
				job={wait.job}
				module={wait.module}
				chainedId={wait.chainedId}
				graphStep={wait.graphStep}
				{isOwner}
				onSelectStep={() => onSelectStep(wait.graphStep)}
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
