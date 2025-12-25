<script lang="ts" module>
	export interface TriggerRunnableConfig {
		path: string
		kind: RunnableKind
		retry?: Retry
		errorHandlerPath?: string
		errorHandlerArgs?: ScriptArgs
	}
</script>

<script lang="ts">
	import Modal2 from '../common/modal/Modal2.svelte'
	import type {
		JobTriggerKind,
		QueuedJob,
		Retry,
		RunnableKind,
		ScriptArgs,
		TriggerMode
	} from '$lib/gen/types.gen'
	import Button from '../common/button/Button.svelte'
	import { workspaceStore } from '$lib/stores'
	import { JobService, TriggerService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import Cell from '$lib/components/table/Cell.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Row from '$lib/components/table/Row.svelte'
	import { Skeleton } from '../common'
	import { Play, Trash2 } from 'lucide-svelte'
	import { displayDate, getJobKindIcon } from '$lib/utils'
	import Alert from '../common/alert/Alert.svelte'
	import { twMerge } from 'tailwind-merge'
	import Badge from '../common/badge/Badge.svelte'
	import Tooltip from '../meltComponents/Tooltip.svelte'
	import { deepEqual } from 'fast-equals'
	import {
		errorHandlerArgs,
		slackErrorHandlerHubPathEnding
	} from '../ErrorOrRecoveryHandler.svelte'

	type Props = {
		triggerPath: string
		triggerKind: JobTriggerKind
		hasChanged: boolean
		onToggleMode: (mode: TriggerMode) => void
		runnableConfig: TriggerRunnableConfig
	}

	let { triggerKind, triggerPath, onToggleMode, hasChanged, runnableConfig }: Props = $props()

	let shouldShowModal = $state(false)
	let queuedJobs = $state<QueuedJob[]>([])
	let selectedJobs = $state<Set<string>>(new Set())
	let loading = $state(false)
	let processingAction = $state(false)
	let workspace = $workspaceStore!
	let currentPage = $state(1)
	let perPage = $state(20)
	let hasMorePages = $derived(queuedJobs.length === perPage)
	let action = $state<'enable' | 'disable' | 'none'>('none')

	// Derived states for checkbox logic
	let allSelected = $derived(queuedJobs.length > 0 && selectedJobs.size === queuedJobs.length)
	let someSelected = $derived(selectedJobs.size > 0 && selectedJobs.size < queuedJobs.length)
	let hasSelectedJobs = $derived(selectedJobs.size > 0)

	$effect(() => {
		if (shouldShowModal) {
			fetchQueuedJobs()
		}
	})

	export async function hasJobs() {
		await fetchQueuedJobs()
		return queuedJobs.length > 0
	}

	export function openModal(newAction: typeof action) {
		action = newAction
		shouldShowModal = true
	}

	export async function fetchQueuedJobs(resetPage = false) {
		if (resetPage) {
			currentPage = 1
		}
		loading = true

		try {
			const allSuspendedJobs = await JobService.listQueue({
				workspace,
				triggerKind,
				jobKinds: 'unassigned_script,unassigned_flow,unassigned_singlestepflow',
				triggerPath,
				running: false,
				perPage,
				page: currentPage
			})

			queuedJobs = allSuspendedJobs
			selectedJobs = new Set()
			return allSuspendedJobs.length
		} catch (e) {
			console.error('Failed to fetch queued jobs:', e)
			sendUserToast(`Failed to fetch queued jobs`, true)
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

		try {
			const jobIds = Array.from(selectedJobs)
			await TriggerService.resumeSuspendedTriggerJobs({
				workspace,
				triggerKind,
				triggerPath,
				requestBody: { job_ids: jobIds }
			})
			sendUserToast(`Started selected jobs`)
			await fetchQueuedJobs()
		} catch (e) {
			console.error('Failed to start selected jobs:', e)
			sendUserToast(`Failed to start selected jobs`, true)
		} finally {
			processingAction = false
		}
	}

	async function discardSelectedJobs() {
		if (selectedJobs.size === 0) return

		processingAction = true

		try {
			const jobIds = Array.from(selectedJobs)
			await TriggerService.cancelSuspendedTriggerJobs({
				workspace,
				triggerKind,
				triggerPath,
				requestBody: { job_ids: jobIds }
			})
			sendUserToast(`Discarded selected jobs`)
			await fetchQueuedJobs()
		} catch (e) {
			console.error('Failed to discard selected jobs:', e)
			sendUserToast(`Failed to discard selected jobs`, true)
		} finally {
			processingAction = false
		}
	}

	async function runAllJobs() {
		if (queuedJobs.length === 0) return

		processingAction = true

		try {
			await TriggerService.resumeSuspendedTriggerJobs({
				workspace,
				triggerKind,
				triggerPath,
				requestBody: {}
			})
			sendUserToast(`Started jobs`)

			if (action === 'enable') {
				onToggleMode('enabled')
			} else if (action === 'disable') {
				onToggleMode('disabled')
			}
			closeModal()
		} catch (e) {
			console.error('Failed to start jobs:', e)
			sendUserToast(`Failed to start jobs`, true)
		} finally {
			processingAction = false
		}
	}

	async function discardAllJobs() {
		if (queuedJobs.length === 0) return

		processingAction = true

		try {
			await TriggerService.cancelSuspendedTriggerJobs({
				workspace,
				triggerKind,
				triggerPath,
				requestBody: {}
			})

			sendUserToast(`Discarded jobs`)

			if (action === 'disable') {
				onToggleMode('disabled')
			} else if (action === 'enable') {
				onToggleMode('enabled')
			} else {
			}

			closeModal()
		} catch (e) {
			console.error('Failed to discard jobs:', e)
			sendUserToast(`Failed to discard jobs`, true)
		} finally {
			processingAction = false
		}
	}

	function closeModal() {
		shouldShowModal = false
	}

	function mapJobKindToRunnableKind(jobKind: QueuedJob['job_kind']): RunnableKind {
		switch (jobKind) {
			case 'unassigned_script':
				return 'script'
			case 'unassigned_flow':
				return 'flow'
			case 'unassigned_singlestepflow':
				return 'script'
			default:
				throw new Error(`Unknown job kind: ${jobKind}`)
		}
	}

	async function checkIfAdvancedOptionsChanged(job: QueuedJob): Promise<boolean> {
		if (job.job_kind === 'unassigned_singlestepflow') {
			if (!runnableConfig.retry && !runnableConfig.errorHandlerPath) {
				return true
			}
			const fullJob = await JobService.getJob({
				workspace,
				id: job.id
			})
			let errorHandlerPath: string | undefined = undefined
			let errorHandlerExtraArgs: ScriptArgs | undefined = undefined
			if (fullJob.raw_flow?.failure_module?.value.type === 'script') {
				errorHandlerPath = fullJob.raw_flow.failure_module.value.path
				const isSlackHandler = errorHandlerPath.endsWith(slackErrorHandlerHubPathEnding)
				errorHandlerExtraArgs = Object.fromEntries(
					Object.entries(fullJob.raw_flow.failure_module.value.input_transforms)
						.filter(
							([key, value]) =>
								!errorHandlerArgs.includes(key) &&
								value.type === 'static' &&
								(!isSlackHandler || key !== 'slack') &&
								value.value !== undefined
						)
						.map(([key, value]) => [
							key,
							(value as Extract<typeof value, { type: 'static' }>).value
						])
				)
			}

			// only keep the retry that is enabled
			const retry = fullJob.raw_flow?.modules[0]?.retry
			if ((retry?.constant?.attempts ?? 0) > 0) {
				delete retry?.exponential
			} else if ((retry?.exponential?.attempts ?? 0) > 0) {
				delete retry?.constant
			}

			const triggerErrorHandlerArgs = runnableConfig.errorHandlerArgs
				? Object.fromEntries(
						Object.entries(runnableConfig.errorHandlerArgs).filter(
							([_, value]) => value !== undefined
						)
					)
				: undefined

			const triggerRetry = structuredClone($state.snapshot(runnableConfig.retry))
			if ((triggerRetry?.constant?.attempts ?? 0) > 0) {
				delete triggerRetry?.exponential
			} else if ((triggerRetry?.exponential?.attempts ?? 0) > 0) {
				delete triggerRetry?.constant
			}

			return (
				!deepEqual(retry, triggerRetry) ||
				!deepEqual(errorHandlerPath, runnableConfig.errorHandlerPath) ||
				!deepEqual(errorHandlerExtraArgs ?? {}, triggerErrorHandlerArgs ?? {})
			)
		} else {
			return runnableConfig.retry !== undefined || runnableConfig.errorHandlerPath !== undefined
		}
	}
</script>

{#snippet runnable({
	path,
	kind,
	outdated
}: {
	path?: string
	kind: QueuedJob['job_kind']
	outdated?: boolean
})}
	{@const JobKindIcon = getJobKindIcon(kind)}
	<div class="flex flex-row gap-2 items-center">
		<Tooltip class="h-full">
			<JobKindIcon size={14} />
			{#snippet text()}
				<span>{mapJobKindToRunnableKind(kind)}</span>
			{/snippet}
		</Tooltip>
		<div
			class={twMerge('whitespace-nowrap text-xs text-primary truncate', outdated && 'line-through')}
			>{path || '-'}</div
		>
	</div>
{/snippet}

<Modal2
	bind:isOpen={shouldShowModal}
	title="{hasMorePages
		? `${queuedJobs.length}+ suspended`
		: `${queuedJobs.length} suspended`} job{queuedJobs.length === 1 ? '' : 's'} for this trigger"
	target="#content"
	fixedHeight="lg"
	fixedWidth="lg"
>
	<div class="flex w-full flex-col gap-4 h-full">
		{#if action !== 'none'}
			<Alert type="warning" title="There are still some suspended jobs">
				You can only {action} the trigger when there are no more suspended jobs for this trigger.
			</Alert>
		{/if}
		{#if queuedJobs.length > 0 && hasChanged}
			<Alert type="warning" title="Trigger has unsaved changes">
				You can only resume suspended jobs when the trigger has no unsaved changes.
			</Alert>
		{:else if queuedJobs.length > 0}
			<Alert type="info" title="Note">
				If you modify the trigger's configuration (such as changing the runnable, retry settings, or
				error handler), resumed jobs will be run using the updated configuration.
			</Alert>
		{/if}

		<div class="relative grow min-h-0 w-full">
			<DataTable
				size="xs"
				paginated
				on:next={nextPage}
				on:previous={prevPage}
				bind:currentPage
				hasMore={hasMorePages}
			>
				<Head>
					<tr>
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
						<Cell head class="w-24">Created at</Cell>
						<Cell head>Runnable</Cell>
						<Cell head class="w-32">Cancellation date</Cell>
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
							{@const changedRunnable =
								runnableConfig.kind !== mapJobKindToRunnableKind(job.job_kind) ||
								runnableConfig.path !== job.script_path}

							<Row
								hoverable
								on:click={() => window.open(`/run/${job.id}?workspace=${workspace}`, '_blank')}
							>
								<Cell class="w-12 sm:pl-3" shouldStopPropagation>
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

								<Cell wrap class="flex flex-row gap-2 items-center">
									{@render runnable({
										path: job.script_path,
										kind: job.job_kind,
										outdated: changedRunnable
									})}
									{#if changedRunnable}
										{@render runnable({
											path: runnableConfig.path,
											kind: runnableConfig.kind
										})}
									{/if}
									{#await checkIfAdvancedOptionsChanged(job) then changedAdvancedOptions}
										{#if changedAdvancedOptions}
											<Badge color="yellow">Changed retry/error handler</Badge>
										{/if}
									{/await}
								</Cell>
								<Cell wrap>{displayDate(job.scheduled_for)}</Cell>
							</Row>
						{/each}
					</tbody>
				{/if}
			</DataTable>
		</div>

		<!-- Bottom right conditional buttons -->
		<div class="flex justify-end gap-2">
			{#if hasSelectedJobs}
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
					startIcon={{ icon: Play }}
					size="sm"
					disabled={processingAction || hasChanged}
					on:click={runSelectedJobs}
				>
					Resume selected ({selectedJobs.size})
				</Button>
			{:else}
				<Button
					startIcon={{ icon: Trash2 }}
					size="sm"
					disabled={processingAction || queuedJobs.length === 0}
					on:click={discardAllJobs}
				>
					Discard all jobs{action === 'disable'
						? ' and disable'
						: action === 'enable'
							? ' and enable trigger'
							: ''}
				</Button>
				<Button
					startIcon={{ icon: Play }}
					size="sm"
					disabled={processingAction || hasChanged || queuedJobs.length === 0}
					on:click={runAllJobs}
				>
					Resume all jobs{action === 'disable'
						? ' and disable'
						: action === 'enable'
							? ' and enable trigger'
							: ''}
				</Button>
			{/if}
		</div>
	</div>
</Modal2>
