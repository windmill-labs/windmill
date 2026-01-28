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
	import { twMerge } from 'tailwind-merge'
	import { untrack } from 'svelte'

	interface Props {
		isOwner: boolean
		workspaceId: string | undefined
		job: Job
		light?: boolean
	}

	let { isOwner, workspaceId, job, light = false }: Props = $props()

	let default_payload: object = $state({})
	let resumeUrl: string | undefined = $state(undefined)
	let cancelUrl: string | undefined = $state(undefined)
	let description: any = $state(undefined)
	let hide_cancel = $state(false)

	let defaultValues = $state({})

	let schema = $state({})
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

	let loading = $state(false)
	async function continu(approve: boolean) {
		loading = true
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
	let approvalStep = $derived((job?.flow_status?.step ?? 1) - 1)
	$effect(() => {
		job && untrack(() => getDefaultArgs())
	})
</script>

<div class="w-full h-full mt-2 text-xs text-primary">
	{#if !light}
		<p>Waiting to be resumed</p>
	{/if}
	{#if description != undefined}
		<DisplayResult {workspaceId} noControls result={description} language={job?.language} />
	{/if}
	<div>
		{#if isOwner || resumeUrl}
			<div class={twMerge('flex gap-2 mt-2', light ? 'flex-col' : 'flex-row ')}>
				{#if cancelUrl && !hide_cancel}
					<div>
						<Button
							title="Cancel the step"
							{loading}
							iconOnly
							startIcon={{ icon: X }}
							variant="default"
							destructive
							on:click={() => continu(false)}
						/>
					</div>
				{/if}
				<div>
					<Button variant="accent" onClick={() => continu(true)} {loading} unifiedSize="md">
						Resume
						<Tooltip class="text-white">
							Since you are an owner of this flow, you can send resume events without necessarily
							knowing the resume id sent by the approval step
						</Tooltip>
					</Button>
				</div>

				{#if job?.raw_flow?.modules?.[approvalStep]?.suspend?.resume_form?.schema}
					<div
						class={twMerge(
							'w-full border rounded-lg p-2',
							light ? 'min-w-96 max-h-svh overflow-y-auto' : ''
						)}
					>
						<SchemaForm onlyMaskPassword bind:args={default_payload} {defaultValues} {schema} />
					</div>
					<Tooltip>
						The payload is optional, it is passed to the following step through the `resume`
						variable
					</Tooltip>
				{/if}
			</div>
		{:else}
			You cannot resume the flow yourself without receiving the resume secret since you are not an
			owner of {job.script_path} and the approval step did not contain the resume url at key `resume`
		{/if}
	</div>
</div>
