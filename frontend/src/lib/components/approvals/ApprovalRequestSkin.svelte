<script lang="ts">
	import type { GetApprovalInfoResponse, Job } from '$lib/gen'
	import { Alert, Badge, Button } from '$lib/components/common'
	import type { BadgeColor } from '$lib/components/common'
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import Login from '$lib/components/Login.svelte'
	import TimeAgo from '$lib/components/TimeAgo.svelte'
	import { enterpriseLicense, userStore } from '$lib/stores'
	import { emptyString } from '$lib/utils'
	import { mergeSchema } from '$lib/common'
	import { CheckCircle2, CircleSlash, ExternalLink, XCircle } from 'lucide-svelte'

	interface Props {
		approvalInfo: GetApprovalInfoResponse & { enums?: Record<string, unknown> }
		job: Job | undefined
		completed: boolean
		actionTaken: 'approved' | 'denied' | undefined
		loading: boolean
		schema: any
		hasForm: boolean
		args: any
		valid: boolean
		isLocked: boolean
		isSelfApprovalBypass: boolean
		isWorkspaceMember: boolean
		runDetailsHref: string
		rd: string
		onApprove: () => void
		onReject: () => void
	}

	let {
		approvalInfo,
		job,
		completed,
		actionTaken,
		loading,
		schema,
		hasForm,
		args = $bindable(),
		valid = $bindable(),
		isLocked,
		isSelfApprovalBypass,
		isWorkspaceMember,
		runDetailsHref,
		rd,
		onApprove,
		onReject
	}: Props = $props()

	type Status = 'pending' | 'approved' | 'rejected' | 'closed'

	const STATUS_BADGE: Record<Status, { label: string; color: BadgeColor }> = {
		pending: { label: 'Pending', color: 'yellow' },
		approved: { label: 'Approved', color: 'green' },
		rejected: { label: 'Rejected', color: 'red' },
		closed: { label: 'Closed', color: 'gray' }
	}

	let status: Status = $derived(
		actionTaken === 'approved'
			? 'approved'
			: actionTaken === 'denied'
				? 'rejected'
				: completed
					? 'closed'
					: 'pending'
	)
	let groupsRequired = $derived(approvalInfo.approval_conditions?.user_groups_required ?? [])
	let isSelfApprovalRefused = $derived(
		!!approvalInfo.approval_conditions?.self_approval_disabled &&
			!!$userStore &&
			$userStore.email === job?.email
	)
</script>

<div class="flex flex-col gap-6">
	<div class="flex flex-col gap-1">
		<div class="flex flex-row items-start justify-between gap-4">
			<span class="text-2xs font-mono font-normal text-emphasis break-all">
				{job?.script_path ?? ''}
			</span>
			<Badge color={STATUS_BADGE[status].color}>{STATUS_BADGE[status].label}</Badge>
		</div>
		{#if job}
			<p class="text-xs font-normal text-secondary">
				Requested by {job.created_by} · <TimeAgo date={job.created_at ?? ''} noSeconds />
			</p>
		{/if}
	</div>

	{#if typeof approvalInfo.description === 'string'}
		<p class="text-xs font-normal text-primary whitespace-pre-wrap">{approvalInfo.description}</p>
	{:else if approvalInfo.description != undefined}
		<DisplayResult noControls result={approvalInfo.description} />
	{/if}

	{#if status === 'pending'}
		{#if hasForm}
			{#if emptyString($enterpriseLicense)}
				<Alert type="warning" title="Adding a form to the approval page is an EE feature" />
			{:else}
				<SchemaForm
					onlyMaskPassword
					noVariablePicker
					bind:isValid={valid}
					schema={mergeSchema(schema, approvalInfo.enums ?? {})}
					bind:args
				/>
			{/if}
		{/if}

		{#if approvalInfo.can_approve}
			<div class="flex flex-row flex-wrap justify-between gap-4">
				{#if approvalInfo.hide_cancel !== true}
					<Button
						unifiedSize="lg"
						variant="default"
						destructive
						onclick={onReject}
						disabled={loading}
					>
						Reject
					</Button>
				{:else}
					<div></div>
				{/if}
				<Button unifiedSize="lg" variant="accent" onclick={onApprove} disabled={!valid || loading}>
					Approve
				</Button>
			</div>
			{#if isSelfApprovalBypass}
				<Alert type="warning" title="Warning">
					As an administrator, by approving or rejecting this request, you bypass the self-approval
					interdiction.
				</Alert>
			{/if}
		{:else if approvalInfo.user_auth_required && !$userStore}
			<p class="text-xs font-normal text-primary">Sign in to review this request.</p>
			<Login {rd} />
		{:else}
			<div class="flex flex-col gap-1 text-xs font-normal text-primary">
				<p>You are not authorized to approve this request.</p>
				{#if isSelfApprovalRefused}
					<p>Self-approval is disabled for this step.</p>
				{/if}
				{#if groupsRequired.length > 0}
					<p>
						Only members of the following groups can approve:
						<span class="font-semibold text-emphasis">{groupsRequired.join(', ')}</span>
					</p>
				{/if}
			</div>
		{/if}
	{:else}
		<div class="flex flex-row items-start gap-3 rounded-md bg-surface-secondary p-4">
			{#if status === 'approved'}
				<CheckCircle2 size={20} class="shrink-0 text-green-500" />
			{:else if status === 'rejected'}
				<XCircle size={20} class="shrink-0 text-red-500" />
			{:else}
				<CircleSlash size={20} class="shrink-0 text-secondary" />
			{/if}
			<div class="flex flex-col gap-1">
				<span class="text-sm font-semibold text-emphasis">
					{status === 'closed' ? 'This request is closed' : STATUS_BADGE[status].label}
				</span>
				<span class="text-xs font-normal text-secondary">
					{#if status === 'approved'}
						Your approval was recorded. You can close this page.
					{:else if status === 'rejected'}
						Your rejection was recorded. You can close this page.
					{:else}
						The flow is no longer waiting for approval.
					{/if}
				</span>
			</div>
		</div>
	{/if}

	{#if !isLocked && ((status === 'pending' && approvalInfo.approvers.length > 0) || isWorkspaceMember)}
		<div
			class="flex flex-row flex-wrap items-center justify-between gap-2 border-t border-border-light pt-4"
		>
			<span class="text-2xs font-normal text-secondary">
				{#if status === 'pending' && approvalInfo.approvers.length > 0}
					Already approved by {approvalInfo.approvers.map((a) => a.approver).join(', ')}
				{/if}
			</span>
			{#if isWorkspaceMember}
				<Button
					unifiedSize="xs"
					variant="subtle"
					href={runDetailsHref}
					target="_blank"
					endIcon={{ icon: ExternalLink }}
				>
					Run details
				</Button>
			{/if}
		</div>
	{/if}
</div>
