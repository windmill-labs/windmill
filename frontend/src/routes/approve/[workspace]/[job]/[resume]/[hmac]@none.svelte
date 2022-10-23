<script lang="ts">
	import { Job, JobService } from '$lib/gen'
	import { page } from '$app/stores'
	import Button from '$lib/components/common/button/Button.svelte'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { sendUserToast } from '$lib/utils'
	import FlowMetadata from '$lib/components/FlowMetadata.svelte'

	let job: Job | undefined = undefined
	let currentApprovers: string[] = []

	getJob()

	async function getJob() {
		const suspendedJobFlow = await JobService.getSuspendedJobFlow({
			workspace: $page.params.workspace,
			id: $page.params.job,
			resumeId: new Number($page.params.resume).valueOf(),
			signature: $page.params.hmac
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
			requestBody: {}
		})
		sendUserToast('Flow approved')
	}

	async function cancel() {
		await JobService.cancelSuspendedJobPost({
			workspace: $page.params.workspace,
			id: $page.params.job,
			resumeId: new Number($page.params.resume).valueOf(),
			signature: $page.params.hmac,
			requestBody: {}
		})
		sendUserToast('Flow disapproved!')
	}
</script>

<div class="min-h-screen antialiased text-gray-900">
	<CenteredModal title="Approve flow?">
		{#if job}
			<FlowMetadata {job} />
		{/if}

		<div class="w-max-md flex flex-row gap-x-4 gap-y-4 justify-between w-full flex-wrap">
			<Button btnClasses="grow" color="red" on:click={cancel} size="md">Disapprove/Cancel</Button>
			<Button btnClasses="grow" on:click={resume} size="md">Approve/Resume</Button>
		</div>

		<div class="mt-4"><a href="https://windmill.dev">Learn more about Windmill</a></div>
	</CenteredModal>
</div>
