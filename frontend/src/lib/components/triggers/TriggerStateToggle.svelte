<script lang="ts">
	import Modal2 from '../common/modal/Modal2.svelte'
	import Toggle from '../Toggle.svelte'
	import { JobService } from '$lib/gen/services.gen'
	import type { QueuedJob } from '$lib/gen/types.gen'
	import Button from '../common/button/Button.svelte'
	import { workspaceStore } from '$lib/stores'
	import RunRow from '../runs/RunRow.svelte'
	import '../runs/runs-grid.css'
	type Props = {
		active_mode: boolean
		suspendNumber?: number
		triggerPath?: string
		onRunSuspendedJobs?: (data: { suspendNumber: number; jobIds: string[] }) => void
	}

	let { active_mode = $bindable(), suspendNumber, onRunSuspendedJobs }: Props = $props()

	let wasInUnactiveMode = $state(!active_mode)
	let shouldShowModal = $state(false)

	$effect(() => {
		if (active_mode && wasInUnactiveMode && suspendNumber !== undefined) {
			shouldShowModal = true
		} else if (!active_mode) {
			wasInUnactiveMode = true
			shouldShowModal = false
		}
	})
	let queuedJobs = $state<QueuedJob[]>([])
	let loading = $state(false)
	let error = $state<string | null>(null)
	let processingAction = $state(false)
	let workspace = $workspaceStore!
	let selectedJobIds = $state<string[]>([])
	let containerWidth = $state(1000)
	$effect(() => {
		if (shouldShowModal && suspendNumber !== undefined) {
			fetchQueuedJobs()
		}
	})

	async function fetchQueuedJobs() {
		if (!suspendNumber) return

		loading = true
		error = null

		try {
			const allSuspendedJobs = await JobService.listQueue({
				workspace,
				suspended: true,
				perPage: 100
			})

			queuedJobs = allSuspendedJobs.filter((job) => job.suspend === suspendNumber)
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
			if (onRunSuspendedJobs && suspendNumber !== undefined) {
				onRunSuspendedJobs({ suspendNumber, jobIds: queuedJobs.map((j) => j.id) })
			}

			closeModal()
		} catch (e) {
			error = `Failed to run jobs: ${e}`
			console.error('Failed to run jobs:', e)
		} finally {
			processingAction = false
		}
	}

	async function discardAllJobs() {
		if (queuedJobs.length === 0) return

		processingAction = true
		error = null

		try {
			const jobIds = queuedJobs.map((job) => job.id)
			await JobService.cancelSelection({
				workspace,
				requestBody: jobIds
			})

			closeModal()
		} catch (e) {
			error = `Failed to discard jobs: ${e}`
			console.error('Failed to discard jobs:', e)
		} finally {
			processingAction = false
		}
	}

	function closeModal() {
		wasInUnactiveMode = false
		shouldShowModal = false
	}

	function cancelToggle() {
		active_mode = false
		closeModal()
	}
</script>

<Toggle bind:checked={active_mode} options={{ right: 'Active', left: 'Unactive' }} />

{#if shouldShowModal}
	<Modal2
		bind:isOpen={shouldShowModal}
		title="{queuedJobs.length} job{queuedJobs.length === 1 ? '' : 's'} queued for this trigger"
		target="#content"
		fixedSize="lg"
	>
		<div class="flex flex-col gap-4 h-full">
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
				<div class="text-center py-8 text-gray-500"> No queued jobs found for this trigger. </div>
			{:else}
				<div class="flex-1 overflow-auto">
					<div class="mb-3">
						<h3 class="text-sm font-medium text-gray-900">Queued Jobs ({queuedJobs.length})</h3>
						<p class="text-xs text-gray-500 mt-1">Click on any job to view details</p>
					</div>

					<!-- Job table container - matching runs page styling -->
					<div class="divide-y h-full border min-w-[650px]" bind:clientWidth={containerWidth}>
						<!-- Table header - using same bg-surface-secondary as runs page -->
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

						<!-- Job rows - no background, let RunRow handle its own styling -->
						<div class="h-full">
							{#each queuedJobs as job}
								<div class="flex flex-row items-center h-[42px] w-full">
									<RunRow
										{job}
										{containerWidth}
										showTag={false}
										activeLabel={null}
										on:select={() => {
											// Handle job selection - navigate to job details or show modal
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
				<Button variant="border" size="sm" on:click={cancelToggle} disabled={processingAction}>
					Cancel
				</Button>

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
