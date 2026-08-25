<script lang="ts">
	import { mergeSchema } from '$lib/common'
	import { type Job, JobService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { ExternalLink, X } from 'lucide-svelte'
	import DisplayResult from './DisplayResult.svelte'
	import Tooltip from './Tooltip.svelte'
	import { Button } from './common'
	import SchemaForm from './SchemaForm.svelte'
	import { twMerge } from 'tailwind-merge'
	import { untrack } from 'svelte'
	import { isReplaying } from './recording/offlineReplay.svelte'

	interface Props {
		isOwner: boolean
		/** The workspace the job ran in — pass `job.workspace_id`, never the navigation
		 *  workspace, which differs whenever the editor is embedded. */
		workspaceId: string | undefined
		job: Job
		light?: boolean
		/** Fired after a successful resume/reject, before the next poll observes it —
		 * lets a host (e.g. the AI chat jobs tray) close its modal optimistically. */
		onAction?: (approved: boolean) => void
	}

	let { isOwner: _isOwner, workspaceId, job, light = false, onAction }: Props = $props()

	let default_payload: object = $state({})
	let description: any = $state(undefined)
	let hide_cancel = $state(false)
	let approvalPageUrl: string | undefined = $state(undefined)

	let defaultValues = $state({})

	let schema = $state({})
	let lastJobId: string | undefined = undefined
	async function getDefaultArgs() {
		let jobId = job?.flow_status?.modules?.[approvalStep]?.job

		if (jobId === lastJobId) {
			return
		}
		if (!jobId || !workspaceId) {
			return {}
		}
		lastJobId = jobId
		let job_result = (await JobService.getCompletedJobResult({
			workspace: workspaceId,
			id: jobId
		})) as any
		const args = job_result?.default_args ?? {}
		description = job_result?.description
		defaultValues = JSON.parse(JSON.stringify(args))
		default_payload = args

		approvalPageUrl = job_result?.['approvalPage']
		actionTaken = false
		hide_cancel = job?.raw_flow?.modules?.[approvalStep]?.suspend?.hide_cancel ?? false
		schema = mergeSchema(
			job?.raw_flow?.modules?.[approvalStep]?.suspend?.resume_form?.schema ?? {},
			job_result?.enums ?? {}
		)
	}

	let loading = $state(false)
	let actionTaken = $state(false)
	async function continu(approve: boolean) {
		if (!workspaceId) {
			sendUserToast('Cannot resume: the job has no workspace', true)
			return
		}
		loading = true
		try {
			await JobService.resumeSuspended({
				workspace: workspaceId,
				jobId: job?.id ?? '',
				requestBody: {
					payload: approve ? (default_payload as any) : undefined,
					approved: approve
				}
			})
			actionTaken = true
			onAction?.(approve)
		} catch (e: any) {
			sendUserToast(e?.body ?? e?.message ?? 'Failed', true)
		} finally {
			loading = false
		}
	}
	let approvalStep = $derived((job?.flow_status?.step ?? 1) - 1)
	// Everything this panel shows (description, resume form enums, approval page) is
	// fetched from the suspended job — a recording carries none of it — and Resume /
	// Cancel act on a job that only exists in the recording. So a replay states the
	// recorded fact and offers nothing to click.
	let replaying = $derived(isReplaying())
	$effect(() => {
		if (!replaying) {
			job && untrack(() => getDefaultArgs())
		}
	})
</script>

{#if replaying}
	<div class="w-full text-xs text-secondary">This step was waiting for approval.</div>
{:else}
	<div class="w-full h-full text-xs text-primary">
		{#if description != undefined}
			<DisplayResult {workspaceId} noControls result={description} language={job?.language} />
			<div class="mt-2"></div>
		{/if}
		<div>
			<div class={twMerge('flex gap-2 items-center', light ? 'flex-col' : 'flex-row ')}>
				{#if !hide_cancel}
					<div>
						<Button
							title="Cancel the step"
							iconOnly
							startIcon={{ icon: X }}
							variant="default"
							disabled={loading || actionTaken}
							destructive
							unifiedSize="md"
							on:click={() => continu(false)}
						/>
					</div>
				{/if}
				<div>
					<Button
						variant="accent"
						onClick={() => continu(true)}
						disabled={loading || actionTaken}
						unifiedSize="md"
					>
						Resume
						<Tooltip class="text-white">Resume or approve this suspended step</Tooltip>
					</Button>
				</div>

				{#if approvalPageUrl}
					<a
						href={approvalPageUrl}
						target="_blank"
						rel="noreferrer"
						class="text-accent flex items-center gap-1 whitespace-nowrap"
					>
						Approval page <ExternalLink size={12} />
					</a>
				{/if}

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
		</div>
	</div>
{/if}
