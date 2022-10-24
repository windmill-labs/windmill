<script lang="ts">
	import { Job, JobService } from '$lib/gen'
	import { page } from '$app/stores'
	import Button from '$lib/components/common/button/Button.svelte'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { sendUserToast } from '$lib/utils'
	import FlowMetadata from '$lib/components/FlowMetadata.svelte'
	import FlowViewer from '$lib/components/FlowViewer.svelte'
	import JobArgs from '$lib/components/JobArgs.svelte'

	let job: Job | undefined = undefined
	let currentApprovers: string[] = []

	getJob()

	async function getJob() {
		const suspendedJobFlow = await JobService.getSuspendedJobFlow({
			workspace: $page.params.workspace,
			id: $page.params.job,
			resumeId: new Number($page.params.resume).valueOf(),
			signature: $page.params.hmac,
			approver: $page.url.searchParams.get('approver') ?? undefined
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
			approver: $page.url.searchParams.get('approver') ?? undefined,
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
			approver: $page.url.searchParams.get('approver') ?? undefined,
			requestBody: {}
		})
		sendUserToast('Flow disapproved!')
		getJob()
	}
</script>

<div class="min-h-screen antialiased text-gray-900">
	<CenteredModal title="Approve flow?">
		<JobArgs {job} />

		{#if job && job.raw_flow}
			<FlowMetadata {job} />
			<FlowViewer flow={{ summary: '', value: job.raw_flow }} />
		{/if}

		<div class="w-max-md flex flex-row gap-x-4 gap-y-4 justify-between w-full flex-wrap">
			<Button btnClasses="grow" color="red" on:click|once={cancel} size="md"
				>Disapprove/Cancel</Button
			>
			<Button btnClasses="grow" on:click|once={resume} size="md">Approve/Resume</Button>
		</div>

		<div class="mt-4"><a href="https://windmill.dev">Learn more about Windmill</a></div>
	</CenteredModal>
</div>
