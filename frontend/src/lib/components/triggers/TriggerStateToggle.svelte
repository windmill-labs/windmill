<script lang="ts">
	import Modal2 from '../common/modal/Modal2.svelte'
	import Toggle from '../Toggle.svelte'
	import type { QueuedJob } from '$lib/gen/types.gen'
	import Button from '../common/button/Button.svelte'
	import { workspaceStore } from '$lib/stores'
	import RunRow from '../runs/RunRow.svelte'
	import '../runs/runs-grid.css'
	import { JobService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { type JobTriggerType } from './utils'
	type Props = {
		active_mode: boolean
		triggerPath: string
		jobTriggerKind: JobTriggerType
	}

	let { active_mode = $bindable(), jobTriggerKind, triggerPath }: Props = $props()

	let wasInInactiveMode = $state(!active_mode)
	let shouldShowModal = $state(false)

	$effect(() => {
		if (active_mode && wasInInactiveMode) {
			shouldShowModal = true
		} else if (!active_mode) {
			wasInInactiveMode = true
			shouldShowModal = false
		}
	})
	let queuedJobs = $state<QueuedJob[]>([])
	let loading = $state(false)
	let error = $state<string | null>(null)
	let processingAction = $state(false)
	let workspace = $workspaceStore!
	let containerWidth = $state(1000)
	$effect(() => {
		if (shouldShowModal) {
			fetchQueuedJobs()
		}
	})

	async function fetchQueuedJobs() {
		loading = true
		error = null

		try {
			const allSuspendedJobs = await JobService.listQueue({
				workspace,
				triggerKind: jobTriggerKind,
				triggerPath,
				running: false,
				perPage: 100
			})

			queuedJobs = allSuspendedJobs
		} catch (e) {
			error = `Failed to fetch queued jobs: ${e}`
			console.error('Failed to fetch queued jobs:', e)
		} finally {
			loading = false
		}
	}

	async function runAllJobs() {
		if (queuedJobs.length === 0) return

		processingAction = true
		error = null

		try {
			const resumedJobs = await JobService.resumeSuspendedJobs({
				workspace,
				requestBody: {
					trigger_kind: jobTriggerKind,
					trigger_path: triggerPath
				}
			})
			sendUserToast(resumedJobs)
		} catch (e) {
			error = `Failed to run jobs: ${e}`
			console.error('Failed to run jobs:', e)
		} finally {
			processingAction = false

			closeModal()
		}
	}

	async function discardAllJobs() {
		if (queuedJobs.length === 0) return

		processingAction = true
		error = null

		try {
			await JobService.cancelSuspendedJobs({
				workspace,
				requestBody: {
					trigger_kind: jobTriggerKind,
					trigger_path: triggerPath
				}
			})

			sendUserToast(`Successfully canceled all jobs`)
		} catch (e) {
			error = `Failed to discard jobs: ${e}`
			console.error('Failed to discard jobs:', e)
		} finally {
			processingAction = false
			closeModal()
		}
	}

	function closeModal() {
		wasInInactiveMode = false
		shouldShowModal = false
	}
</script>

<Toggle bind:checked={active_mode} options={{ right: 'Active', left: 'Inactive' }} />

{#if shouldShowModal}
	<Modal2
		bind:isOpen={shouldShowModal}
		title="{queuedJobs.length} job{queuedJobs.length === 1 ? '' : 's'} queued for this trigger"
		target="#content"
		fixedSize="lg"
	>
		<div class="flex w-full flex-col gap-4 h-full">
			{#if loading}
				<div class="flex items-center justify-center py-8">
					<div
						class="animate-spin h-6 w-6 border-2 border-blue-500 border-t-transparent rounded-full"
					></div>
					<span class="ml-2">Loading queued jobs...</span>
				</div>
			{:else if error}
				<div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded">
					{error}
				</div>
			{:else if queuedJobs.length === 0}
				<div class="flex flex-col items-center w-full py-12 px-4">
					<div class="text-center">
						<div class="text-base font-medium text-secondary mb-2">No queued jobs found</div>
						<div class="text-sm text-tertiary"
							>This trigger has no jobs waiting to be processed.</div
						>
					</div>
				</div>
			{:else}
				<div class="flex-1 overflow-auto">
					<div class="mb-3">
						<h3 class="text-sm font-medium">Queued Jobs ({queuedJobs.length})</h3>
						<p class="text-xs text-gray-500 mt-1">Click on any job to view details</p>
					</div>

					<div class="divide-y h-full border min-w-[650px]" bind:clientWidth={containerWidth}>
						<div
							class="bg-surface-secondary sticky top-0 w-full py-2 pr-4 grid grid-runs-table-no-tag"
						>
							<div class="text-2xs px-2 font-semibold">Status</div>
							<div class="text-xs font-semibold">Started</div>
							<div class="text-xs font-semibold">Duration</div>
							<div class="text-xs font-semibold">Path</div>
							<div class="text-xs font-semibold">Triggered by</div>
							<div class=""></div>
						</div>

						<div class="h-full">
							{#each queuedJobs as job}
								<div class="flex flex-row items-center h-[42px] w-full">
									<RunRow
										{job}
										{containerWidth}
										showTag={false}
										activeLabel={null}
										on:select={() => {
											window.open(`/run/${job.id}?workspace=${workspace}`, '_blank')
										}}
									/>
								</div>
							{/each}
						</div>
					</div>
				</div>

				<div class="bg-blue-50 p-4 rounded-lg">
					<p class="text-sm text-blue-700">
						You are switching this trigger from inactive to active mode. What would you like to do
						with the {queuedJobs.length} queued job{queuedJobs.length === 1 ? '' : 's'}?
					</p>
				</div>
			{/if}

			<div class="flex gap-2 pt-4 border-t">
				{#if !loading && !error && queuedJobs.length > 0}
					<Button
						variant="border"
						size="sm"
						on:click={discardAllJobs}
						disabled={processingAction}
						color="red"
					>
						{processingAction ? 'Discarding...' : 'Discard All Jobs'}
					</Button>

					<Button
						variant="contained"
						size="sm"
						on:click={runAllJobs}
						disabled={processingAction}
						color="green"
					>
						{processingAction ? 'Running...' : 'Run All Jobs'}
					</Button>
				{/if}
			</div>
		</div>
	</Modal2>
{/if}
