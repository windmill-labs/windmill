<script lang="ts">
	import { mergeSchema } from '$lib/common'
	import { type Job, JobService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { X } from 'lucide-svelte'
	import DisplayResult from './DisplayResult.svelte'
	import Tooltip from './Tooltip.svelte'
	import { Button } from './common'
	import SchemaForm from './SchemaForm.svelte'

	export let isOwner: boolean
	export let workspaceId: string | undefined
	export let job: Job

	let default_payload: object = {}
	let resumeUrl: string | undefined = undefined
	let cancelUrl: string | undefined = undefined
	let description: any = undefined
	let hide_cancel = false

	$: approvalStep = (job?.flow_status?.step ?? 1) - 1

	let defaultValues = {}
	$: job && getDefaultArgs()

	let schema = {}
	let lastJobId: string | undefined = undefined
	async function getDefaultArgs() {
		let jobId = job?.flow_status?.modules?.[approvalStep]?.job

		if (jobId === lastJobId) {
			return
		}
		if (!jobId) {
			return {}
		}
		lastJobId = jobId
		let job_result = (await JobService.getCompletedJobResult({
			workspace: workspaceId ?? $workspaceStore ?? '',
			id: jobId
		})) as any
		const args = job_result?.default_args ?? {}
		description = job_result?.description
		defaultValues = JSON.parse(JSON.stringify(args))
		default_payload = args

		resumeUrl = job_result?.['resume']
		cancelUrl = job_result?.['cancel']
		hide_cancel = job?.raw_flow?.modules?.[approvalStep]?.suspend?.hide_cancel ?? false
		schema = mergeSchema(
			job?.raw_flow?.modules?.[approvalStep]?.suspend?.resume_form?.schema ?? {},
			job_result?.enums ?? {}
		)
	}

	async function continu(approve: boolean) {
		if ((resumeUrl && approve) || (cancelUrl && !approve)) {
			let split = (approve ? resumeUrl : cancelUrl)!.split('/')
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
			if (approve) {
				await JobService.resumeSuspendedJobPost({
					workspace: workspaceId ?? $workspaceStore ?? '',
					id: jobId,
					requestBody: default_payload as any,
					resumeId,
					signature,
					approver
				})
			} else {
				await JobService.cancelSuspendedJobPost({
					workspace: workspaceId ?? $workspaceStore ?? '',
					id: jobId,
					resumeId,
					signature,
					approver,
					requestBody: {}
				})
			}
		} else {
			if (approve) {
				await JobService.resumeSuspendedFlowAsOwner({
					workspace: workspaceId ?? $workspaceStore ?? '',
					id: job?.id ?? '',
					requestBody: default_payload as any
				})
			} else {
				await JobService.cancelQueuedJob({
					workspace: workspaceId ?? $workspaceStore ?? '',
					id: job?.id ?? '',
					requestBody: {}
				})
			}
		}
	}
</script>

<div class="w-full h-full mt-2 text-sm text-tertiary">
	<p>Waiting to be resumed</p>
	{#if description != undefined}
		<DisplayResult {workspaceId} noControls result={description} language={job?.language}/>
	{/if}
	<div>
		{#if isOwner || resumeUrl}
			<div class="flex flex-row gap-2 mt-2">
				{#if cancelUrl && !hide_cancel}
					<div>
						<Button
							color="red"
							title="Cancel the flow"
							iconOnly
							startIcon={{ icon: X }}
							variant="border"
							on:click={() => continu(false)}
						/>
					</div>
				{/if}
				<div>
					<Button color="green" variant="border" on:click={() => continu(true)}
						>Resume <Tooltip
							>Since you are an owner of this flow, you can send resume events without necessarily
							knowing the resume id sent by the approval step</Tooltip
						></Button
					>
				</div>

				{#if job?.raw_flow?.modules?.[approvalStep]?.suspend?.resume_form?.schema}
					<div class="w-full border rounded-lg p-2">
						<SchemaForm onlyMaskPassword bind:args={default_payload} {defaultValues} {schema} />
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
