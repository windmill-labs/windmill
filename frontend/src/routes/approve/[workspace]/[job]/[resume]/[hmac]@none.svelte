<script lang="ts">
	import { Job, JobService } from '$lib/gen'
	import { page } from '$app/stores'
	import Button from '$lib/components/common/button/Button.svelte'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { sendUserToast } from '$lib/utils'
	import FlowMetadata from '$lib/components/FlowMetadata.svelte'
	import FlowModulesViewer from '$lib/components/FlowModulesViewer.svelte'
	import JobArgs from '$lib/components/JobArgs.svelte'
	import { onMount } from 'svelte'

	let job: Job | undefined = undefined
	let currentApprovers: string[] = []
	let approver = $page.url.searchParams.get('approver') ?? undefined

	let completed: boolean = false
	$: completed = job?.type == 'CompletedJob'

	getJob()

	onMount(() => {
		window.onunhandledrejection = (event: PromiseRejectionEvent) => {
			event.preventDefault()

			if (event.reason?.message) {
				const { message, body, status } = event.reason

				if (body) {
					sendUserToast(`${body}`, true)
				} else {
					sendUserToast(`${message}`, true)
				}
			} else {
				console.log('Caught unhandled promise rejection without message', event)
			}
		}
	})

	async function getJob() {
		const suspendedJobFlow = await JobService.getSuspendedJobFlow({
			workspace: $page.params.workspace,
			id: $page.params.job,
			resumeId: new Number($page.params.resume).valueOf(),
			signature: $page.params.hmac,
			approver
		})
		job = suspendedJobFlow.job
		currentApprovers = suspendedJobFlow.approvers
	}

	async function resume() {
		await JobService.resumeSuspendedJobPost({
			workspace: $page.params.workspace,
			id: $page.params.job,
			resumeId: new Number($page.params.resume).valueOf(),
			signature: $page.params.hmac,
			approver,
			requestBody: {}
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
		sendUserToast('Flow disapproved!')
		getJob()
	}
</script>

<div class="min-h-screen antialiased text-gray-900">
	<CenteredModal title="Approve resuming of flow?">
		<div class="flex flex-row justify-between flex-wrap sm:flex-nowrap gap-x-4">
			<div class="w-full">
				<h2 class="mt-4">Current approvers</h2>
				<div class="my-4">
					{#if currentApprovers.length > 0}
						{currentApprovers.join(', ')}
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
					<FlowMetadata {job} />
				{/if}
			</div>
		</div>
		<h2 class="mt-4">Flow arguments</h2>

		<JobArgs {job} />
		<div class="mt-8">
			{#if approver}
				<p>Dis/approving as: <b>{approver}</b></p>
			{/if}
		</div>
		{#if completed}
			<div class="my-2"><p><b>The flow is already completed</b></p></div>
		{/if}

		<div class="w-max-md flex flex-row gap-x-4 gap-y-4 justify-between w-full flex-wrap mt-2">
			<Button btnClasses="grow" color="red" on:click|once={cancel} size="md" disabled={completed}
				>Disapprove/Cancel</Button
			>
			<Button btnClasses="grow" on:click|once={resume} size="md" disabled={completed}
				>Approve/Resume</Button
			>
		</div>

		<div class="mt-4"><a href="https://windmill.dev">Learn more about Windmill</a></div>
		{#if job && job.raw_flow}
			<h2 class="mt-10">Flow details</h2>
			<FlowModulesViewer
				modules={job.raw_flow?.modules}
				failureModule={job.raw_flow?.failure_module}
			/>
		{/if}
	</CenteredModal>
</div>
