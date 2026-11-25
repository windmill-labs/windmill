<script lang="ts">
	import { resource } from 'runed'
	import { ChevronDown, ExternalLink, Hourglass } from 'lucide-svelte'
	import { Badge, Button } from '$lib/components/common'
	import { JobService, type FlowModule, type Job } from '$lib/gen'
	import FlowStatusWaitingForEvents from '../FlowStatusWaitingForEvents.svelte'
	import StepIdBadge from '../flows/content/StepIdBadge.svelte'
	import { displayDate } from '$lib/utils'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'

	interface Props {
		/** The run the step belongs to: the page's run, or a subflow, branch or loop run under it. */
		job: Job
		module: FlowModule
		isOwner: boolean
		/** The step's id, chained after the subflow steps leading to it (`outer:r:s`). */
		chainedId: string
		/** The page run's step the graph selects: the step itself, or the one holding its run. */
		graphStep: string
		onSelectStep: () => void
	}

	let { job, module, isOwner, chainedId, graphStep, onSelectStep }: Props = $props()

	// Primitives: each poll brings a new `job` object, which would refetch on every poll.
	let workspace = $derived(job.workspace_id)
	let jobId = $derived(job.id)
	let stepId = $derived(module.id)
	const detail = resource(
		() => ({ workspace, jobId, stepId }),
		async ({ workspace, jobId, stepId }) =>
			workspace ? JobService.getApprovalStepDetail({ workspace, jobId, stepId }) : undefined
	)

	// Open for whoever can act on it; a long description or form takes room only for them.
	let toggled = $state<boolean | undefined>(undefined)
	let expanded = $derived(toggled ?? detail.current?.can_approve ?? isOwner)

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

	const namesLabel = (names: string[]) =>
		names.slice(0, 2).join(', ') + (names.length > 2 ? ` +${names.length - 2}` : '')

	let toggleLabel = $derived(expanded ? 'Hide the approval form' : 'Show the approval form')
</script>

<div class="flex items-start gap-3 px-4 py-2 border-t text-xs">
	<Hourglass size={14} class="shrink-0 mt-[3px] text-violet-500" />
	<div class="flex flex-1 min-w-0 flex-wrap items-center gap-x-3 gap-y-0.5">
		<span class="flex flex-wrap items-center gap-1 min-w-0 w-64 max-w-full -ml-2">
			<Button
				variant="subtle"
				unifiedSize="xs"
				btnClasses="justify-start min-w-0 gap-1"
				title="Show {graphStep} in the graph"
				onclick={onSelectStep}
			>
				<span class="truncate">{module.summary || module.id}</span>
				{#if module.summary || chainedId !== module.id}
					<StepIdBadge id={chainedId} maxLength={24} />
				{/if}
			</Button>
		</span>
		<span
			class="flex flex-1 basis-48 flex-wrap items-center gap-x-3 gap-y-0.5 min-w-0 text-secondary"
		>
			{#if detail.current}
				<span><span class="text-hint">Can approve:</span> {canApproveLabel()}</span>
				{@const approvals = detail.current.approvals}
				{#if approvals.length > 0}
					<!-- Two names keep the row on one line; all of them, with times, are on hover. -->
					<Tooltip small>
						{#snippet text()}
							{#each approvals as a}
								<div>
									{a.approver}{#if a.created_at}<span class="opacity-70"
											>{` · ${displayDate(a.created_at)}`}</span
										>{/if}
								</div>
							{/each}
						{/snippet}
						<span>
							{approvals.length} of {detail.current.required_events} approvals:
							<span class="text-primary">{namesLabel(approvals.map((a) => a.approver))}</span>
						</span>
					</Tooltip>
				{/if}
			{/if}
			<!-- The page the Home list and links sent through other channels open, for this run. -->
			<Button
				variant="subtle"
				unifiedSize="xs"
				href="/approve/{workspace ?? ''}/{jobId}"
				target="_blank"
				endIcon={{ icon: ExternalLink }}
			>
				Approval page
			</Button>
		</span>
	</div>
	<span class="flex shrink-0 items-center gap-1">
		<Badge color="violet" small>Waiting</Badge>
		<Button
			variant="subtle"
			unifiedSize="xs"
			iconOnly
			startIcon={{ icon: ChevronDown, classes: expanded ? 'rotate-180' : '' }}
			title={toggleLabel}
			aria-label={toggleLabel}
			aria-expanded={expanded}
			onclick={() => (toggled = !expanded)}
		/>
	</span>
</div>
{#if expanded}
	<!-- The step's own approval panel, unchanged: its description, form and buttons. -->
	<div class="px-4 pt-1 pb-3">
		<!-- A partial approval changes nothing the job's updates carry: refetch the count. -->
		<FlowStatusWaitingForEvents
			workspaceId={job.workspace_id}
			{job}
			{isOwner}
			onAction={() => detail.refetch()}
		/>
	</div>
{/if}
