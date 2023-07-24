<script lang="ts">
	import { Job, JobService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import SchemaForm from './SchemaForm.svelte'
	import Tooltip from './Tooltip.svelte'
	import { Button } from './common'

	export let is_owner: boolean
	export let workspaceId: string | undefined
	export let job: Job

	let payload: object = {}

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
		payload = job_result?.default_args ?? {}
	}
</script>

<div class="w-full h-full mt-2 text-sm text-tertiary">
	<p>Waiting to be resumed</p>
	<div>
		{#if is_owner}
			<div class="flex flex-row gap-2 mt-2">
				<div>
					<Button
						color="green"
						variant="border"
						on:click={async () =>
							await JobService.resumeSuspendedFlowAsOwner({
								workspace: workspaceId ?? $workspaceStore ?? '',
								id: job?.id ?? '',
								requestBody: payload
							})}
						>Resume <Tooltip
							>Since you are an owner of this flow, you can send resume events without necessarily
							knowing the resume id sent by the approval step</Tooltip
						></Button
					>
				</div>
				{#if job.raw_flow?.modules?.[approvalStep]?.suspend?.resume_form?.schema}
					<div class="w-full border rounded-lg p-2">
						<SchemaForm
							noVariablePicker
							bind:args={payload}
							schema={job.raw_flow?.modules?.[approvalStep]?.suspend?.resume_form?.schema}
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
