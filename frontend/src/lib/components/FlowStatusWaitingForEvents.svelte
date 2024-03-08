<script lang="ts">
	import { mergeSchema } from '$lib/common'
	import { Job, JobService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import LightweightSchemaForm from './LightweightSchemaForm.svelte'
	import Tooltip from './Tooltip.svelte'
	import { Button } from './common'

	export let isOwner: boolean
	export let workspaceId: string | undefined
	export let job: Job

	let default_payload: object = {}
	let enum_payload: object = {}
	let resumeUrl: string | undefined = undefined

	$: approvalStep = (job?.flow_status?.step ?? 1) - 1

	$: job && getDefaultArgs()

	async function getDefaultArgs() {
		let jobId = job.flow_status?.modules?.[approvalStep]?.job
		if (!jobId) {
			return {}
		}
		let job_result = await JobService.getCompletedJobResult({
			workspace: workspaceId ?? $workspaceStore ?? '',
			id: jobId
		})
		default_payload = job_result?.default_args ?? {}
		enum_payload = job_result?.enums ?? {}
		resumeUrl = job_result?.['resume']
	}
</script>

<div class="w-full h-full mt-2 text-sm text-tertiary">
	<p>Waiting to be resumed</p>
	<div>
		{#if isOwner || resumeUrl}
			<div class="flex flex-row gap-2 mt-2">
				<div>
					<Button
						color="green"
						variant="border"
						on:click={async () => {
							if (resumeUrl) {
								let split = resumeUrl.split('/')
								let signatureUrl = split.pop() ?? ''
								const regex = /([^?]+)(?:\?[^=]+=(\w+))?/

								const matches = signatureUrl.match(regex)

								const signature = matches?.[1]
								if (!signature) {
									sendUserToast(`Could not parse signature: ${signatureUrl}`, true)
									return
								}
								const approver = matches?.[2] || undefined

								let resumeId = -1
								let parsedResumeId = split.pop() ?? ''
								try {
									resumeId = new Number(parsedResumeId).valueOf()
								} catch (e) {
									console.error(`Could not parse resume id: ${parsedResumeId}`)
								}
								let jobId = split.pop() ?? ''
								await JobService.resumeSuspendedJobPost({
									workspace: workspaceId ?? $workspaceStore ?? '',
									id: jobId,
									requestBody: default_payload,
									resumeId,
									signature,
									approver
								})
							} else {
								await JobService.resumeSuspendedFlowAsOwner({
									workspace: workspaceId ?? $workspaceStore ?? '',
									id: job?.id ?? '',
									requestBody: default_payload
								})
							}
						}}
						>Resume <Tooltip
							>Since you are an owner of this flow, you can send resume events without necessarily
							knowing the resume id sent by the approval step</Tooltip
						></Button
					>
				</div>

				{#if job.raw_flow?.modules?.[approvalStep]?.suspend?.resume_form?.schema}
					<div class="w-full border rounded-lg p-2">
						<LightweightSchemaForm
							bind:args={default_payload}
							schema={mergeSchema(
								job.raw_flow?.modules?.[approvalStep]?.suspend?.resume_form?.schema ?? {},
								enum_payload
							)}
						/>
					</div>
				{/if}
				<Tooltip
					>The payload is optional, it is passed to the following step through the `resume` variable</Tooltip
				>
			</div>
		{:else}
			You cannot resume the flow yourself without receiving the resume secret since you are not an
			owner of {job.script_path} and the approval step did not contain the resume url at key `resume`
		{/if}
	</div>
</div>
