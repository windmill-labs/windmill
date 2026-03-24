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

		hide_cancel = job?.raw_flow?.modules?.[approvalStep]?.suspend?.hide_cancel ?? false
		schema = mergeSchema(
			job?.raw_flow?.modules?.[approvalStep]?.suspend?.resume_form?.schema ?? {},
			job_result?.enums ?? {}
		)
	}

	let loading = $state(false)
	async function continu(approve: boolean) {
		loading = true
		try {
			await JobService.resumeSuspended({
				workspace: workspaceId ?? $workspaceStore ?? '',
				jobId: job?.id ?? '',
				requestBody: {
					payload: approve ? (default_payload as any) : undefined,
					approved: approve
				}
			})
		} catch (e: any) {
			sendUserToast(e?.body ?? e?.message ?? 'Failed', true)
		} finally {
			loading = false
		}
	}
	let approvalStep = $derived((job?.flow_status?.step ?? 1) - 1)
	$effect(() => {
		job && untrack(() => getDefaultArgs())
	})
</script>

<div class="w-full h-full text-xs text-primary">
	{#if description != undefined}
		<DisplayResult {workspaceId} noControls result={description} language={job?.language} />
		<div class="mt-2"></div>
	{/if}
	<div>
		<div class={twMerge('flex gap-2', light ? 'flex-col' : 'flex-row ')}>
			{#if !hide_cancel}
				<div>
					<Button
						title="Cancel the step"
						iconOnly
						startIcon={{ icon: X }}
						variant="default"
						disabled={loading}
						destructive
						unifiedSize="md"
						on:click={() => continu(false)}
					/>
				</div>
			{/if}
			<div>
				<Button variant="accent" onClick={() => continu(true)} {loading} unifiedSize="md">
					Resume
					<Tooltip class="text-white">Resume or approve this suspended step</Tooltip>
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
					The payload is optional, it is passed to the following step through the `resume` variable
				</Tooltip>
			{/if}
		</div>
	</div>
</div>
