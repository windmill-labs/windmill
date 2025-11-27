<script lang="ts">
	import Modal2 from '../common/modal/Modal2.svelte'
	import type { QueuedJob } from '$lib/gen/types.gen'
	import Button from '../common/button/Button.svelte'
	import { workspaceStore } from '$lib/stores'
	import RunRow from '../runs/RunRow.svelte'
	import '../runs/runs-grid.css'
	import { JobService, TriggerService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { type JobTriggerType } from './utils'
	import { ChevronLeft, ChevronRight } from 'lucide-svelte'
	type Props = {
		suspended_mode: boolean
		triggerPath: string
		jobTriggerKind: JobTriggerType
	}

	let { suspended_mode = $bindable(), jobTriggerKind, triggerPath }: Props = $props()

	let wasInInactiveMode = $state(!suspended_mode)
	let shouldShowModal = $state(false)

	$effect(() => {
		if (suspended_mode && wasInInactiveMode) {
			shouldShowModal = true
		} else if (!suspended_mode) {
			wasInInactiveMode = true
			shouldShowModal = false
		}
	})
	let queuedJobs = $state<QueuedJob[]>([])
	let loading = $state(false)
	let error = $state<string | null>(null)
	let processingAction = $state(false)
	let isPreviousLoading = $state(false)
	let isNextLoading = $state(false)
	let workspace = $workspaceStore!
	let containerWidth = $state(1000)
	let currentPage = $state(1)
	let perPage = $state(20)
	let hasMorePages = $derived(queuedJobs.length === perPage)
	$effect(() => {
		if (shouldShowModal) {
			fetchQueuedJobs()
		}
	})

	async function fetchQueuedJobs(resetPage = false) {
		if (resetPage) {
			currentPage = 1
		}
		loading = true
		error = null

		try {
			const allSuspendedJobs = await JobService.listQueue({
				workspace,
				triggerKind: jobTriggerKind,
				jobKinds: 'unassigned',
				triggerPath,
				running: false,
				perPage,
				page: currentPage
			})

			queuedJobs = allSuspendedJobs
		} catch (e) {
			error = `Failed to fetch queued jobs: ${e}`
			console.error('Failed to fetch queued jobs:', e)
		} finally {
			loading = false
		}
	}

	function nextPage() {
		if (hasMorePages && !loading) {
			isNextLoading = true
			currentPage += 1
			fetchQueuedJobs().finally(() => {
				isNextLoading = false
			})
		}
	}

	function prevPage() {
		if (currentPage > 1 && !loading) {
			isPreviousLoading = true
			currentPage -= 1
			fetchQueuedJobs().finally(() => {
				isPreviousLoading = false
			})
		}
	}

	async function runAllJobs() {
		if (queuedJobs.length === 0) return

		processingAction = true
		error = null

		try {
			const resumedJobs = await TriggerService.resumeSuspendedTriggerJobs({
				workspace,
				triggerKind: jobTriggerKind,
				triggerPath
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
			await TriggerService.cancelSuspendedTriggerJobs({
				workspace,
				triggerKind: jobTriggerKind,
				triggerPath
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

{#if shouldShowModal}
	<Modal2
		bind:isOpen={shouldShowModal}
		title="{hasMorePages
			? `${queuedJobs.length}+ suspended`
			: `${queuedJobs.length} suspended`} job{queuedJobs.length === 1 ? '' : 's'} for this trigger"
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
						<div class="text-base font-medium text-secondary mb-2">No suspended jobs found</div>
						<div class="text-sm text-tertiary"
							>This trigger has no suspended jobs waiting to be processed.</div
						>
					</div>
				</div>
			{:else}
				<div class="flex-1 overflow-auto">
					<div class="mb-3">
						<h3 class="text-sm font-medium">
							Suspended Jobs {#if hasMorePages}(Page {currentPage}){:else}({queuedJobs.length}){/if}
						</h3>
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

				{#if queuedJobs.length > 0 && (currentPage > 1 || hasMorePages)}
					<div
						class="w-full bg-surface border-t flex flex-row justify-between p-2 items-center gap-2"
					>
						<div class="flex flex-row gap-2 items-center">
							<span class="text-xs text-secondary">
								{queuedJobs.length}
								{hasMorePages ? '+' : ''} suspended job{queuedJobs.length === 1 ? '' : 's'}
							</span>
						</div>

						<div class="flex flex-row gap-3 items-center">
							<div class="flex text-xs text-secondary">Page {currentPage}</div>

							<Button
								variant="subtle"
								size="xs2"
								startIcon={{ icon: ChevronLeft }}
								on:click={prevPage}
								disabled={currentPage === 1 || loading}
								loading={isPreviousLoading}
							>
								Previous
							</Button>

							<Button
								variant="subtle"
								size="xs2"
								endIcon={{ icon: ChevronRight }}
								on:click={nextPage}
								disabled={!hasMorePages || loading}
								loading={isNextLoading}
							>
								Next
							</Button>
						</div>
					</div>
				{/if}

				<div class="bg-blue-50 p-4 rounded-lg">
					<p class="text-sm text-blue-700">
						You are switching this trigger from inactive to active mode. What would you like to do
						with the {hasMorePages
							? `${queuedJobs.length}+ suspended`
							: `${queuedJobs.length} suspended`} job{queuedJobs.length === 1 ? '' : 's'}?
					</p>
				</div>
			{/if}

			<div class="flex gap-2 pt-4 border-t">
				{#if !loading && !error && queuedJobs.length > 0}
					<Button
						variant="border"
						size="sm"
						onClick={discardAllJobs}
						disabled={processingAction}
						color="red"
					>
						{processingAction ? 'Discarding...' : 'Discard All Jobs'}
					</Button>

					<Button
						variant="contained"
						size="sm"
						onClick={runAllJobs}
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
