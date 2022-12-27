<script lang="ts">
	import { Job, JobService } from '$lib/gen'
	import { page } from '$app/stores'
	import Button from '$lib/components/common/button/Button.svelte'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { sendUserToast } from '$lib/utils'
	import FlowMetadata from '$lib/components/FlowMetadata.svelte'
	import JobArgs from '$lib/components/JobArgs.svelte'
	import { onDestroy, onMount } from 'svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import FlowGraph from '$lib/components/graph/FlowGraph.svelte'
	import autosize from 'svelte-autosize'

	let job: Job | undefined = undefined
	let currentApprovers: { resume_id: number; approver: string }[] = []
	let approver = $page.url.searchParams.get('approver') ?? undefined

	let completed: boolean = false
	$: completed = job?.type == 'CompletedJob'
	$: alreadyResumed = currentApprovers
		.map((x) => x.resume_id)
		.includes(new Number($page.params.resume).valueOf())

	let timeout: NodeJS.Timer | undefined = undefined
	let error: string | undefined = undefined
	let payload = ''

	onMount(() => {
		getJob()
		timeout = setInterval(getJob, 1000)
		window.onunhandledrejection = (event: PromiseRejectionEvent) => {
			event.preventDefault()
			timeout && clearInterval(timeout)
			if (event.reason?.message) {
				const { message, body, status } = event.reason

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
	})

	onDestroy(() => {
		timeout && clearInterval(timeout)
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
			requestBody: payload
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
	<CenteredModal title="Approval for resuming of flow">
		{#if error}
			<h1 class="text-red-400">{error}</h1>
		{/if}
		<div class="flex flex-row justify-between flex-wrap sm:flex-nowrap gap-x-4">
			<div class="w-full">
				<h2 class="mt-4">Current approvers</h2>
				<p class="text-xs italic"
					>Each approver can only approve once and cannot change his approver name set by the
					approval sender</p
				>
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
					<FlowMetadata {job} />
				{/if}
			</div>
		</div>
		<h2 class="mt-4">Flow arguments</h2>

		<JobArgs args={job?.args} />
		<div class="mt-8">
			{#if approver}
				<p>Dis/approving as: <b>{approver}</b></p>
			{/if}
		</div>
		{#if completed}
			<div class="my-2"
				><p><b>The flow is not running anymore. You cannot cancel or resume it.</b></p></div
			>
		{:else if alreadyResumed}
			<div class="my-2"><p><b>You have already approved this flow to be resumed</b></p></div>
		{/if}

		<div class="w-max-md flex flex-row gap-x-4 gap-y-4 justify-between w-full flex-wrap mt-2">
			<Button
				btnClasses="grow"
				color="red"
				on:click|once={cancel}
				size="md"
				disabled={completed || alreadyResumed}>Disapprove/Cancel</Button
			>
			<Button
				btnClasses="grow"
				color="green"
				on:click|once={resume}
				size="md"
				disabled={completed || alreadyResumed}>Approve/Resume</Button
			>
		</div>
		<div>
			<h3 class="mt-2">Payload (optional)</h3>
			<input type="text" bind:value={payload} use:autosize />
		</div>

		<div class="mt-4 flex flex-row flex-wrap justify-between"
			><a href="https://windmill.dev">Learn more about Windmill</a>
			<a target="_blank" rel="noreferrer" href="/run/{job?.id}?workspace={job?.workspace_id}"
				>Flow run details (require auth)</a
			>
		</div>
		{#if job && job.raw_flow}
			<h2 class="mt-10">Flow details</h2>
			<div class="border border-gray-700">
				<FlowGraph
					modules={job.raw_flow?.modules}
					failureModule={job.raw_flow?.failure_module}
					notSelectable
				/>
			</div>
		{/if}
	</CenteredModal>
</div>
