<script lang="ts">
	import Modal2 from '../common/modal/Modal2.svelte'
	import type { JobTriggerKind, QueuedJob } from '$lib/gen/types.gen'
	import Button from '../common/button/Button.svelte'
	import { workspaceStore } from '$lib/stores'
	import { HttpTriggerService, JobService, TriggerService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import Cell from '$lib/components/table/Cell.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Row from '$lib/components/table/Row.svelte'
	import { Skeleton } from '../common'
	import { PlayCircle, Trash2 } from 'lucide-svelte'
	import { displayDate } from '$lib/utils'
	import Badge from '../common/badge/Badge.svelte'
	import Tooltip from '../Tooltip.svelte'

	type Props = {
		suspendedMode: boolean
		triggerPath: string
		jobTriggerKind: JobTriggerKind
		shouldShowModal: boolean
		editedAt: string | undefined
		onTriggerEnabled?: () => void
		onToggleSuspendedMode?: (suspendedMode: boolean, enabled?: boolean) => void
	}

	let {
		jobTriggerKind,
		triggerPath,
		shouldShowModal = $bindable(),
		onTriggerEnabled,
		editedAt
	}: Props = $props()

	let queuedJobs = $state<QueuedJob[]>([])
	let selectedJobs = $state<Set<string>>(new Set())
	let loading = $state(false)
	let error = $state<string | null>(null)
	let processingAction = $state(false)
	let workspace = $workspaceStore!
	let currentPage = $state(1)
	let perPage = $state(20)
	let hasMorePages = $derived(queuedJobs.length === perPage)
	let headerHeight = $state(0)
	let contentHeight = $state(0)

	// Derived states for checkbox logic
	let allSelected = $derived(queuedJobs.length > 0 && selectedJobs.size === queuedJobs.length)
	let someSelected = $derived(selectedJobs.size > 0 && selectedJobs.size < queuedJobs.length)
	let hasSelectedJobs = $derived(selectedJobs.size > 0)

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
			selectedJobs = new Set()
		} catch (e) {
			error = `Failed to fetch queued jobs: ${e}`
			console.error('Failed to fetch queued jobs:', e)
		} finally {
			loading = false
		}
	}

	function nextPage() {
		if (hasMorePages && !loading) {
			currentPage += 1
			fetchQueuedJobs()
		}
	}

	function prevPage() {
		if (currentPage > 1 && !loading) {
			currentPage -= 1
			fetchQueuedJobs()
		}
	}

	function toggleSelectAll() {
		if (allSelected) {
			selectedJobs = new Set()
		} else {
			selectedJobs = new Set(queuedJobs.map((job) => job.id))
		}
	}

	function toggleJobSelection(jobId: string) {
		const newSelection = new Set(selectedJobs)
		if (newSelection.has(jobId)) {
			newSelection.delete(jobId)
		} else {
			newSelection.add(jobId)
		}
		selectedJobs = newSelection
	}

	async function runSelectedJobs() {
		if (selectedJobs.size === 0) return

		processingAction = true
		error = null

		try {
			const jobIds = Array.from(selectedJobs)
			await TriggerService.resumeSuspendedTriggerJobs({
				workspace,
				triggerKind: jobTriggerKind,
				triggerPath,
				requestBody: { job_ids: jobIds }
			})
			sendUserToast(`Successfully resumed ${jobIds.length} job${jobIds.length > 1 ? 's' : ''}`)
			await fetchQueuedJobs()
		} catch (e) {
			error = `Failed to run selected jobs: ${e}`
			console.error('Failed to run selected jobs:', e)
		} finally {
			processingAction = false
		}
	}

	async function discardSelectedJobs() {
		if (selectedJobs.size === 0) return

		processingAction = true
		error = null

		try {
			const jobIds = Array.from(selectedJobs)
			await TriggerService.cancelSuspendedTriggerJobs({
				workspace,
				triggerKind: jobTriggerKind,
				triggerPath,
				requestBody: { job_ids: jobIds }
			})
			sendUserToast(`Successfully canceled ${jobIds.length} job${jobIds.length > 1 ? 's' : ''}`)
			await fetchQueuedJobs()
		} catch (e) {
			error = `Failed to discard selected jobs: ${e}`
			console.error('Failed to discard selected jobs:', e)
		} finally {
			processingAction = false
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
				triggerPath,
				requestBody: {}
			})

			//TODO: Add support for other trigger types
			await HttpTriggerService.updateHttpTriggerStatus({
				workspace,
				path: triggerPath,
				requestBody: { enabled: true, suspended_mode: false }
			})
			sendUserToast(resumedJobs)
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
			await TriggerService.cancelSuspendedTriggerJobs({
				workspace,
				triggerKind: jobTriggerKind,
				triggerPath,
				requestBody: {}
			})

			await HttpTriggerService.updateHttpTriggerStatus({
				workspace,
				path: triggerPath,
				requestBody: { enabled: false, suspended_mode: false }
			})

			sendUserToast(`Successfully canceled all jobs`)
			closeModal()
		} catch (e) {
			error = `Failed to discard jobs: ${e}`
			console.error('Failed to discard jobs:', e)
		} finally {
			processingAction = false
		}
	}

	function enableTrigger() {
		onTriggerEnabled?.()
		closeModal()
	}

	function closeModal() {
		shouldShowModal = false
	}
</script>

<Modal2
	zIndex={1102}
	bind:isOpen={shouldShowModal}
	title="{hasMorePages
		? `${queuedJobs.length}+ suspended`
		: `${queuedJobs.length} suspended`} job{queuedJobs.length === 1 ? '' : 's'} for this trigger"
	target="#content"
	fixedSize="lg"
>
	<div class="flex w-full flex-col gap-4 h-full">
		{#if error}
			<div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded">
				{error}
			</div>
		{/if}

		<div class="relative grow min-h-0 w-full">
			<DataTable
				size="xs"
				paginated
				on:next={nextPage}
				on:previous={prevPage}
				bind:currentPage
				hasMore={hasMorePages}
				bind:contentHeight
			>
				<Head>
					<tr bind:clientHeight={headerHeight}>
						<Cell head first class="w-12">
							<div class="h-4 w-4">
								<input
									type="checkbox"
									checked={allSelected}
									indeterminate={someSelected}
									onchange={toggleSelectAll}
									disabled={loading || queuedJobs.length === 0}
									class="cursor-pointer w-auto"
								/>
							</div>
						</Cell>
						<Cell head class="min-w-24">Created At</Cell>
						<Cell head class="min-w-32 ">Script/Flow Path</Cell>
					</tr>
				</Head>
				{#if loading}
					<tbody>
						{#each new Array(3) as _}
							<Row>
								{#each new Array(4) as _}
									<Cell>
										<Skeleton layout={[[5]]} />
									</Cell>
								{/each}
							</Row>
						{/each}
					</tbody>
				{:else if queuedJobs.length === 0}
					<div class="absolute top-0 left-0 w-full h-full center-center">
						<p class="text-center text-gray-500 mt-4">No suspended jobs found.</p>
					</div>
				{:else}
					<tbody class="divide-y border-b w-full overflow-y-auto">
						{#each queuedJobs as job}
							<Row>
								<Cell class="w-12 sm:pl-3">
									<div class="h-4 w-4">
										<input
											type="checkbox"
											checked={selectedJobs.has(job.id)}
											onchange={() => toggleJobSelection(job.id)}
											disabled={processingAction}
											class="cursor-pointer"
										/>
									</div>
								</Cell>

								<Cell wrap>{displayDate(job.created_at)}</Cell>

								<Cell wrap class="flex flex-row gap-2">
									<a
										href="/run/{job.id}?workspace={workspace}"
										target="_blank"
										class="text-blue-600 hover:underline"
									>
										<div class="flex-shrink min-w-0 break-words">{job.script_path || '-'}</div>
									</a>
									{#if editedAt && job.created_at && new Date(editedAt) > new Date(job.created_at)}
										<Badge color="yellow"
											>Outdated <Tooltip>
												Trigger was edited after these jobs were created and suspended. They will be
												reassigned to match the new trigger configuration, in particular the
												runnable, retry and error handling settings.
											</Tooltip>
										</Badge>
									{/if}
								</Cell>
							</Row>
						{/each}
					</tbody>
				{/if}
			</DataTable>
		</div>

		<!-- Bottom right conditional buttons -->
		<div class="flex justify-end gap-2">
			{#if queuedJobs.length === 0}
				<!-- No jobs found - show Enable Trigger -->
				<Button size="sm" disabled={processingAction} on:click={enableTrigger}>
					Enable Trigger
				</Button>
				<Button size="sm" disabled={processingAction} on:click={enableTrigger}>
					Enable Trigger
				</Button>
			{:else if hasSelectedJobs}
				<!-- Jobs selected - show Discard Selected and Run Selected -->
				<Button
					startIcon={{ icon: Trash2 }}
					size="sm"
					disabled={processingAction}
					on:click={discardSelectedJobs}
				>
					Discard Selected ({selectedJobs.size})
				</Button>
				<Button
					startIcon={{ icon: PlayCircle }}
					size="sm"
					disabled={processingAction}
					on:click={runSelectedJobs}
				>
					Run selected ({selectedJobs.size})
				</Button>
			{:else}
				<Button
					startIcon={{ icon: Trash2 }}
					size="sm"
					disabled={processingAction}
					on:click={discardAllJobs}
				>
					Discard all jobs and disable
				</Button>
				<Button
					startIcon={{ icon: PlayCircle }}
					size="sm"
					disabled={processingAction}
					on:click={runAllJobs}
				>
					Run all jobs and resume
				</Button>
			{/if}
		</div>
	</div>
</Modal2>
