<script lang="ts">
	import { resource } from 'runed'
	import { ChevronDown, Hourglass } from 'lucide-svelte'
	import { Badge, Button } from '$lib/components/common'
	import { JobService, type FlowModule, type Job } from '$lib/gen'
	import FlowStatusWaitingForEvents from '../FlowStatusWaitingForEvents.svelte'
	import StepIdBadge from '../flows/content/StepIdBadge.svelte'

	interface Props {
		/** The run the step belongs to: the page's run, or a subflow, branch or loop run under it. */
		job: Job
		module: FlowModule
		isOwner: boolean
		/** The page run's step that holds a nested wait. */
		within?: FlowModule
		onSelectStep: () => void
	}

	let { job, module, isOwner, within, onSelectStep }: Props = $props()

	const detail = resource(
		() => ({ workspace: job.workspace_id, jobId: job.id, stepId: module.id }),
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

	let toggleLabel = $derived(expanded ? 'Hide the approval form' : 'Show the approval form')
</script>

<div
	class="grid grid-cols-[16px_minmax(140px,1fr)_minmax(0,2fr)_auto] items-center gap-3 px-4 py-2 border-t text-xs"
>
	<Hourglass size={14} class="text-violet-500" />
	<span class="flex flex-wrap items-center gap-1 min-w-0">
		<Button
			variant="subtle"
			unifiedSize="xs"
			btnClasses="justify-start min-w-0 truncate"
			title="Show {(within ?? module).summary || (within ?? module).id} in the graph"
			onclick={onSelectStep}
		>
			{module.summary || module.id}
		</Button>
		{#if module.summary}<StepIdBadge id={module.id} />{/if}
		{#if within}
			<span class="text-hint">in</span>
			<span class="truncate text-secondary">{within.summary || within.id}</span>
			{#if within.summary}<StepIdBadge id={within.id} />{/if}
		{/if}
	</span>
	<span class="flex flex-wrap items-center gap-x-3 gap-y-0.5 min-w-0 text-secondary">
		{#if detail.current}
			<span><span class="text-hint">Can approve:</span> {canApproveLabel()}</span>
			{#if detail.current.approvals.length > 0}
				<span>{detail.current.approvals.length} of {detail.current.required_events} approvals</span>
			{/if}
		{/if}
	</span>
	<span class="flex items-center gap-1">
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
		<FlowStatusWaitingForEvents workspaceId={job.workspace_id} {job} {isOwner} />
	</div>
{/if}
