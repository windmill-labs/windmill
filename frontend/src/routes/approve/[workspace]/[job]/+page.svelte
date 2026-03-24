<script lang="ts">
	import { type Job, JobService } from '$lib/gen'
	import { base } from '$lib/base'
	import Button from '$lib/components/common/button/Button.svelte'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { sendUserToast } from '$lib/toast'
	import FlowMetadata from '$lib/components/FlowMetadata.svelte'
	import JobArgs from '$lib/components/JobArgs.svelte'
	import { onDestroy, onMount, untrack } from 'svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import { enterpriseLicense, userStore, workspaceStore } from '$lib/stores'
	import Login from '$lib/components/Login.svelte'
	import { LogIn, AlertTriangle, ExternalLink } from 'lucide-svelte'
	import { mergeSchema } from '$lib/common'
	import { emptyString } from '$lib/utils'
	import { Alert } from '$lib/components/common'
	import { getUserExt } from '$lib/user'
	import { setLicense } from '$lib/enterpriseUtils'
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import ScheduleEditor from '$lib/components/triggers/schedules/ScheduleEditor.svelte'
	import FlowGraphV2 from '$lib/components/graph/FlowGraphV2.svelte'
	import { page } from '$app/state'

	$workspaceStore = page.params.workspace
	let rd = page.url.href
	let token = page.url.searchParams.get('token') ?? undefined

	let job: Job | undefined = $state(undefined)
	let approvalInfo: any = $state(undefined)
	let completed = $state(false)
	let error: string | undefined = $state(undefined)
	let default_payload: any = $state({})
	let loading = $state(false)
	let valid = $state(true)

	let pollInterval: number | undefined = undefined
	let scheduleEditor: ScheduleEditor | undefined = $state(undefined)

	setLicense()

	onMount(() => {
		window.onunhandledrejection = (event: PromiseRejectionEvent) => {
			event.preventDefault()
			pollInterval && clearInterval(pollInterval)
			if (event.reason?.message) {
				const { message, body } = event.reason
				if (body) {
					sendUserToast(`${body}`, true)
					error = body.toString()
				} else {
					sendUserToast(`${message}`, true)
					error = message.toString()
				}
			}
		}
		loadData()
		pollInterval = setInterval(loadData, 2000)
	})

	onDestroy(() => {
		pollInterval && clearInterval(pollInterval)
	})

	async function loadData() {
		try {
			const [info, jobData] = await Promise.all([
				JobService.getApprovalInfo({
					workspace: page.params.workspace ?? '',
					jobId: page.params.job ?? '',
					token
				}),
				JobService.getJob({
					workspace: page.params.workspace ?? '',
					id: page.params.job ?? ''
				})
			])
			approvalInfo = info
			job = jobData as Job
			completed = job?.type === 'CompletedJob'
		} catch (e: any) {
			error = e?.body ?? e?.message ?? 'Failed to load approval info'
			pollInterval && clearInterval(pollInterval)
		}
	}

	async function loadUser() {
		userStore.set(await getUserExt(page.params.workspace ?? ''))
	}

	async function resume() {
		loading = true
		try {
			await JobService.resumeSuspended({
				workspace: page.params.workspace ?? '',
				jobId: page.params.job ?? '',
				requestBody: {
					payload: default_payload,
					approval_token: token,
					approved: true
				}
			})
			sendUserToast('Flow approved')
			loadData()
		} catch (e: any) {
			sendUserToast(e?.body ?? e?.message ?? 'Failed to approve', true)
			error = e?.body ?? e?.message
		} finally {
			loading = false
		}
	}

	async function cancel() {
		loading = true
		try {
			await JobService.resumeSuspended({
				workspace: page.params.workspace ?? '',
				jobId: page.params.job ?? '',
				requestBody: {
					approval_token: token,
					approved: false
				}
			})
			sendUserToast('Flow denied!')
			loadData()
		} catch (e: any) {
			sendUserToast(e?.body ?? e?.message ?? 'Failed to cancel', true)
			error = e?.body ?? e?.message
		} finally {
			loading = false
		}
	}

	let isWac = $derived(!!(job as any)?.workflow_as_code_status)
	let filteredArgs = $derived.by(() => {
		if (!job?.args) return job?.args
		const args = { ...(job.args as any) }
		delete args['_MODULES']
		return args
	})
	let rawFormSchema = $derived(approvalInfo?.form_schema?.schema ?? approvalInfo?.form_schema ?? {})
	let schema = $derived.by(() => {
		if (
			!rawFormSchema ||
			typeof rawFormSchema !== 'object' ||
			Object.keys(rawFormSchema).length === 0
		)
			return {}
		// If the schema already has 'properties', use it as-is (classic flow format)
		if ('properties' in rawFormSchema) return rawFormSchema
		// Otherwise wrap as properties (WAC format: form.schema is a flat map of field definitions)
		return { properties: rawFormSchema, order: Object.keys(rawFormSchema) }
	})
	let hasForm = $derived(schema && typeof schema === 'object' && Object.keys(schema).length > 0)
	let selfApprovalDisabled = $derived(
		approvalInfo?.approval_conditions?.self_approval_disabled ?? false
	)
	let isSelfApprovalBypass = $derived(
		selfApprovalDisabled &&
			$userStore &&
			$userStore.email === (job as any)?.email &&
			($userStore.is_admin || $userStore.is_super_admin)
	)

	$effect(() => {
		if (approvalInfo?.user_auth_required && !$userStore) {
			untrack(() => loadUser())
		}
	})
</script>

<ScheduleEditor bind:this={scheduleEditor} />

<CenteredModal
	title="Approval for resuming of {isWac ? 'workflow' : 'flow'}"
	disableLogo
	centerVertically={false}
>
	{#if error}
		<div class="space-y-6">
			{#if error.includes('logged in') || error.includes('sign in') || error.includes('Not authorized')}
				<div class="flex flex-row gap-4 justify-center">
					<AlertTriangle />
					<p class="text-lg">Not Authorized</p>
				</div>
				<p class="text-sm">{error}</p>
				<Login {rd} />
			{:else if error.includes('Permission denied') || error.includes('Self-approval')}
				<div class="flex flex-row gap-4 justify-center">
					<AlertTriangle />
					<p class="text-lg">Permission denied</p>
				</div>
				<p class="text-sm">{error}</p>
			{:else}
				<div class="flex flex-row gap-4 justify-center">
					<AlertTriangle />
					<p class="text-lg">Error</p>
				</div>
				<p class="text-sm">{error}</p>
			{/if}
		</div>
	{:else if approvalInfo}
		<div class="flex flex-row justify-between flex-wrap sm:flex-nowrap gap-x-4">
			<div class="w-full">
				<h2 class="text-sm font-semibold text-emphasis">Approvers</h2>
				<div class="mt-2 text-xs font-normal text-primary">
					{#if approvalInfo.approvers?.length > 0}
						<ul>
							{#each approvalInfo.approvers as a}
								<li>
									<p>
										{a.approver}
										<Tooltip>Unique id of approval: {a.resume_id}</Tooltip>
									</p>
								</li>
							{/each}
						</ul>
					{:else}
						<p class="text-xs text-secondary">
							No current approvers for this step (approval steps can require more than one approval)
						</p>
					{/if}
				</div>
			</div>
			<div class="w-full">
				{#if job && job.raw_flow}
					<FlowMetadata {job} {scheduleEditor} />
				{/if}
			</div>
		</div>

		{#if !completed}
			<h2 class="mt-4 mb-2 text-sm font-semibold text-emphasis">
				{isWac ? 'Workflow' : 'Flow'} arguments
			</h2>
			<JobArgs
				id={job?.id}
				workspace={job?.workspace_id ?? $workspaceStore ?? 'no_w'}
				args={filteredArgs}
			/>
		{/if}

		<div class="mt-8"></div>

		<div class="p-4 rounded-md bg-surface-tertiary shadow-md">
			{#if completed}
				<Alert type="info" title="Flow completed">
					The flow is not running anymore. You cannot cancel or resume it.
				</Alert>
			{/if}

			{#if approvalInfo.description != undefined}
				<DisplayResult noControls result={approvalInfo.description} />
			{/if}

			{#if hasForm && !completed}
				{#if emptyString($enterpriseLicense)}
					<Alert type="warning" title="Adding a form to the approval page is an EE feature" />
				{:else}
					<SchemaForm
						onlyMaskPassword
						noVariablePicker
						bind:isValid={valid}
						schema={mergeSchema(schema, {})}
						bind:args={default_payload}
					/>
				{/if}
			{/if}

			{#if !completed && approvalInfo.can_approve}
				<div class="w-max-md flex flex-row gap-x-4 gap-y-4 justify-between w-full flex-wrap">
					{#if approvalInfo.hide_cancel !== true}
						<Button
							variant="accent"
							destructive
							onclick={cancel}
							size="lg"
							disabled={completed || loading}
						>
							Deny
						</Button>
					{:else}
						<div></div>
					{/if}
					<Button
						variant="accent"
						onclick={resume}
						size="lg"
						disabled={completed || !valid || loading}
					>
						Approve
					</Button>
				</div>
			{:else if !completed && !approvalInfo.can_approve}
				{#if approvalInfo.user_auth_required && !$userStore}
					<Login {rd} />
				{:else}
					<div class="text-sm text-secondary space-y-1">
						<p>You are not authorized to approve this flow.</p>
						{#if approvalInfo.approval_conditions?.self_approval_disabled && $userStore && $userStore.email === (job as any)?.email}
							<p class="text-yellow-600">Self-approval is disabled for this step.</p>
						{/if}
						{#if approvalInfo.approval_conditions?.user_groups_required?.length > 0}
							<p
								>Only members of the following groups can approve: <b
									>{approvalInfo.approval_conditions.user_groups_required.join(', ')}</b
								></p
							>
						{/if}
					</div>
				{/if}
			{:else if completed}
				<!-- already shown above -->
			{/if}

			{#if !completed && isSelfApprovalBypass}
				<div class="mt-2">
					<Alert type="warning" title="Warning">
						As an administrator, by resuming or cancelling this stage of the flow, you bypass the
						self-approval interdiction.
					</Alert>
				</div>
			{/if}
		</div>

		<div class="mt-4 flex flex-row flex-wrap justify-between">
			<a
				class="text-accent text-xs"
				target="_blank"
				rel="noreferrer"
				href="{base}/run/{job?.id}?workspace={job?.workspace_id}"
			>
				Open run details (require auth) <ExternalLink size={12} class="inline" />
			</a>
		</div>

		{#if job && job.raw_flow && !completed}
			<h2 class="mt-10 text-sm font-semibold text-emphasis mb-2">Flow details</h2>
			<div class="rounded-md overflow-hidden">
				<FlowGraphV2
					workspace={job.workspace_id}
					triggerNode={false}
					earlyStop={job.raw_flow?.skip_expr !== undefined}
					cache={job.raw_flow?.cache_ttl !== undefined}
					modules={job.raw_flow?.modules}
					failureModule={job.raw_flow?.failure_module}
					preprocessorModule={job.raw_flow?.preprocessor_module}
					notSelectable
				/>
			</div>
		{/if}
	{:else}
		<p class="text-sm text-secondary">Loading...</p>
	{/if}
</CenteredModal>
