<script lang="ts">
	import { mergeSchema } from '$lib/common'
	import { Job, JobService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import LightweightSchemaForm from './LightweightSchemaForm.svelte'
	import Tooltip from './Tooltip.svelte'
	import { Button } from './common'

	export let isOwner: boolean
	export let workspaceId: string | undefined
	export let job: Job

	let default_payload: object = {}
	let enum_payload: object = {}

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
	}
</script>

<div class="w-full h-full mt-2 text-sm text-tertiary">
	<p>Waiting to be resumed</p>
	<div>
		{#if isOwner}
			<div class="flex flex-row gap-2 mt-2">
				<div>
					<Button
						color="green"
						variant="border"
						on:click={async () =>
							await JobService.resumeSuspendedFlowAsOwner({
								workspace: workspaceId ?? $workspaceStore ?? '',
								id: job?.id ?? '',
								requestBody: default_payload
							})}
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
			owner of {job.script_path}
		{/if}
	</div>
</div>
