<script lang="ts">
	import { type Job, JobService } from '$lib/gen'
	import { page } from '$app/stores'
	import { base } from '$lib/base'
	import Button from '$lib/components/common/button/Button.svelte'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { sendUserToast } from '$lib/toast'
	import FlowMetadata from '$lib/components/FlowMetadata.svelte'
	import JobArgs from '$lib/components/JobArgs.svelte'
	import { onDestroy, onMount } from 'svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import { enterpriseLicense, userStore, workspaceStore } from '$lib/stores'
	import { LogIn, AlertTriangle } from 'lucide-svelte'
	import { mergeSchema } from '$lib/common'
	import { emptyString } from '$lib/utils'
	import { Alert } from '$lib/components/common'
	import { getUserExt } from '$lib/user'
	import { setLicense } from '$lib/enterpriseUtils'
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import ScheduleEditor from '$lib/components/triggers/schedules/ScheduleEditor.svelte'
	import FlowGraphV2 from '$lib/components/graph/FlowGraphV2.svelte'

	$workspaceStore = $page.params.workspace
	let rd = $page.url.href.replace($page.url.origin, '')

	let job: Job | undefined = $state(undefined)
	let currentApprovers: { resume_id: number; approver: string }[] = $state([])
	let approver = $page.url.searchParams.get('approver') ?? undefined

	let completed: boolean = $state(false)

	let dynamicSchema: any = $state({})

	let timeout: NodeJS.Timeout | undefined = undefined
	let error: string | undefined = $state(undefined)
	let default_payload: any = $state.raw({})
	let enum_payload: object = $state({})
	let description: any = $state(undefined)

	setLicense()

	onMount(() => {
		window.onunhandledrejection = (event: PromiseRejectionEvent) => {
			event.preventDefault()
			timeout && clearInterval(timeout)
			if (event.reason?.message) {
				const { message, body } = event.reason

				if (body) {
					sendUserToast(`${body}`, true)
					error = body.toString()
				} else {
					sendUserToast(`${message}`, true)
					error = message.toString()
				}
			} else {
				console.log('Caught unhandled promise rejection without message', event)
			}
		}
		getJob()
		timeout = setInterval(getJob, 1000)
	})

	onDestroy(() => {
		timeout && clearInterval(timeout)
	})

	let argsFetched = $state(false)

	let valid = $state(true)
	async function getDefaultArgs() {
		argsFetched = true
		let jobId = job?.flow_status?.modules?.[approvalStep]?.job
		if (!jobId) {
			return {}
		}
		let job_result = (await JobService.getCompletedJobResult({
			workspace: job?.workspace_id ?? '',
			id: jobId,
			secret: $page.params.hmac,
			suspendedJob: $page.params.job,
			resumeId: new Number($page.params.resume).valueOf(),
			approver
		})) as any
		description = job_result?.description
		default_payload = job_result?.default_args ?? {}
		enum_payload = job_result?.enums ?? {}
		if (job_result?.schema) {
			dynamicSchema = job_result?.schema ?? {}
		}
	}

	async function getJob() {
		const suspendedJobFlow = await JobService.getSuspendedJobFlow({
			workspace: $page.params.workspace,
			id: $page.params.job,
			resumeId: new Number($page.params.resume).valueOf(),
			signature: $page.params.hmac,
			approver
		})
		job = suspendedJobFlow.job as Job
		currentApprovers = suspendedJobFlow.approvers
	}

	async function resume() {
		await JobService.resumeSuspendedJobPost({
			workspace: $page.params.workspace,
			id: $page.params.job,
			resumeId: new Number($page.params.resume).valueOf(),
			signature: $page.params.hmac,
			approver,
			requestBody: default_payload
		})
		sendUserToast('Flow approved')
		getJob()
	}

	async function cancel() {
		await JobService.cancelSuspendedJobPost({
			workspace: $page.params.workspace,
			id: $page.params.job,
			resumeId: new Number($page.params.resume).valueOf(),
			signature: $page.params.hmac,
			approver,
			requestBody: {}
		})
		sendUserToast('Flow denied!')
		getJob()
	}

	async function loadUser() {
		userStore.set(await getUserExt($page.params.workspace))
	}

	let scheduleEditor: ScheduleEditor | undefined = $state(undefined)
	$effect(() => {
		completed = job?.type == 'CompletedJob'
	})
	let alreadyResumed = $derived(
		currentApprovers.map((x) => x.resume_id).includes(new Number($page.params.resume).valueOf())
	)
	let approvalStep = $derived.by(() => (job?.flow_status?.step ?? 1) - 1)
	let schema = $derived.by(
		() => job?.raw_flow?.modules?.[approvalStep]?.suspend?.resume_form?.schema ?? dynamicSchema
	)
	$effect(() => {
		job && !argsFetched && getDefaultArgs()
	})
	$effect(() => {
		if (job?.raw_flow?.modules?.[approvalStep]?.suspend?.user_auth_required && !$userStore) {
			loadUser()
		}
	})
</script>

<ScheduleEditor bind:this={scheduleEditor} />

<CenteredModal title="Approval for resuming of flow" disableLogo>
	{#if error}
		<div class="space-y-6">
			{#if error.startsWith('Not authorized:')}
				<div class="flex flex-row gap-4 justify-center">
					<AlertTriangle />
					<p class="text-lg">Not Authorized</p>
				</div>
				<p class="text-sm">{error.replace(/^(Not authorized: )/, '')}</p>
				<Button href={`/user/login?${rd ? 'rd=' + encodeURIComponent(rd) : ''}`}>
					Sign in
					<LogIn class="w-8" size={18} />
				</Button>
			{:else}
				<div class="flex flex-row gap-4 justify-center">
					<AlertTriangle class="" />
					<p class="text-lg">Permission denied</p>
				</div>
				<p class="text-sm">{error.replace(/^(Permission denied: )/, '')}</p>
			{/if}
		</div>
	{:else}
		<div class="flex flex-row justify-between flex-wrap sm:flex-nowrap gap-x-4">
			<div class="w-full">
				<h2 class="mt-4">Approvers</h2>

				<div class="my-4">
					{#if currentApprovers.length > 0}
						<ul>
							{#each currentApprovers as approver}
								<li
									><b
										>{approver.approver}<Tooltip
											>Unique id of approval: {approver.resume_id}</Tooltip
										></b
									></li
								>
							{/each}
						</ul>
					{:else}
						<p class="text-sm"
							>No current approvers for this step (approval steps can require more than one
							approval)</p
						>
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
			<h2 class="mt-4 mb-2">Flow arguments</h2>

			<JobArgs
				id={job?.id}
				workspace={job?.workspace_id ?? $workspaceStore ?? 'no_w'}
				args={job?.args}
			/>
		{/if}

		<div class="mt-8">
			{#if approver}
				<p>Approving as: <b>{approver}</b></p>
			{/if}
		</div>
		{#if completed}
			<div class="my-2"
				><p><b>The flow is not running anymore. You cannot cancel or resume it.</b></p></div
			>
		{:else if alreadyResumed}
			<div class="my-2"><p><b>You have already approved this flow to be resumed</b></p></div>
		{/if}

		{#if description != undefined}
			<DisplayResult noControls result={description} />
		{/if}

		{#if schema && !completed}
			{#if emptyString($enterpriseLicense)}
				<Alert type="warning" title="Adding a form to the approval page is an EE feature" />
			{:else}
				<SchemaForm
					onlyMaskPassword
					noVariablePicker
					bind:isValid={valid}
					schema={mergeSchema(schema, enum_payload)}
					bind:args={default_payload}
				/>
			{/if}
		{/if}

		{#if !completed}
			<div class="w-max-md flex flex-row gap-x-4 gap-y-4 justify-between w-full flex-wrap mt-2">
				{#if !job?.raw_flow?.modules?.[approvalStep]?.suspend?.hide_cancel}
					<Button
						btnClasses="grow"
						color="red"
						on:click|once={cancel}
						size="md"
						disabled={completed || alreadyResumed}>Deny</Button
					>
				{:else}
					<div></div>
				{/if}

				<Button
					btnClasses="grow"
					color="green"
					on:click|once={resume}
					size="md"
					disabled={completed || alreadyResumed || !valid}>Approve</Button
				>
			</div>
		{/if}
		{#if !completed && !alreadyResumed && job?.raw_flow?.modules?.[approvalStep]?.suspend?.user_auth_required && job?.raw_flow?.modules?.[approvalStep]?.suspend?.self_approval_disabled && $userStore && $userStore.email === job.email && ($userStore.is_admin || $userStore.is_super_admin)}
			<div class="mt-2">
				<Alert type="warning" title="Warning">
					As an administrator, by resuming or cancelling this stage of the flow, you bypass the
					self-approval interdiction.
				</Alert>
			</div>
		{/if}

		<div class="mt-4 flex flex-row flex-wrap justify-between">
			<a target="_blank" rel="noreferrer" href="{base}/run/{job?.id}?workspace={job?.workspace_id}"
				>Flow run details (require auth)</a
			>
		</div>
		{#if job && job.raw_flow && !completed}
			<h2 class="mt-10">Flow details</h2>
			<div class="border border-gray-700">
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
	{/if}
</CenteredModal>
