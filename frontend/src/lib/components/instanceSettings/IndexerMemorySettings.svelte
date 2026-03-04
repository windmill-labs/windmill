<script lang="ts">
	import { Button } from '$lib/components/common'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import { IndexSearchService } from '$lib/gen'
	import type { GetIndexerStatusResponse } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { displaySize } from '$lib/utils'
	import Tooltip from '../Tooltip.svelte'
	import IntegerInput from '../IntegerInput.svelte'
	import InputError from '../InputError.svelte'
	import Label from '../Label.svelte'
	import { Loader2, RefreshCw } from 'lucide-svelte'
	import type { Writable } from 'svelte/store'

	interface Props {
		values: Writable<Record<string, any>>
		disabled?: boolean
		errors?: Record<string, string>
	}

	let { values, disabled = false, errors = {} }: Props = $props()

	let clearJobsIndexModalOpen = $state(false)
	let clearServiceLogsIndexModalOpen = $state(false)

	let status: GetIndexerStatusResponse | undefined = $state(undefined)
	let statusLoading = $state(true)
	let statusError = $state(false)

	function formatTimeAgo(isoDate: string): string {
		const diffMs = Date.now() - new Date(isoDate).getTime()
		const diffSecs = Math.floor(diffMs / 1000)
		if (diffSecs < 60) return `${diffSecs}s ago`
		const diffMins = Math.floor(diffSecs / 60)
		if (diffMins < 60) return `${diffMins}m ago`
		const diffHours = Math.floor(diffMins / 60)
		return `${diffHours}h ago`
	}

	async function loadStatus() {
		statusLoading = true
		statusError = false
		try {
			status = await IndexSearchService.getIndexerStatus()
		} catch (e) {
			status = undefined
			statusError = true
		} finally {
			statusLoading = false
		}
	}

	loadStatus()
</script>

<div class="space-y-6">
	<div class="flex flex-col gap-1">
		<label for="writer_memory_budget" class="block text-xs font-semibold text-emphasis">
			Index writer memory budget (MB)
			<Tooltip>
				The allocated memory arena for the indexer. A bigger value means less writing to disk and
				potentially higher indexing throughput
			</Tooltip>
		</label>
		<IntegerInput
			placeholder="300"
			id="writer_memory_budget"
			{disabled}
			error={errors.writer_memory_budget ?? ''}
			value={$values['indexer_settings'].writer_memory_budget != null
				? $values['indexer_settings'].writer_memory_budget / (1024 * 1024)
				: undefined}
			oninput={(v) => {
				if (v == null) {
					const { writer_memory_budget: _, ...rest } = $values['indexer_settings']
					$values['indexer_settings'] = rest
				} else {
					$values['indexer_settings'] = {
						...$values['indexer_settings'],
						writer_memory_budget: v * (1024 * 1024)
					}
				}
			}}
		/>
		<InputError error={errors.writer_memory_budget ?? ''} />
	</div>
	<Label label="Indexer status">
		{#snippet action()}
			<button
				onclick={loadStatus}
				disabled={statusLoading}
				class="inline-flex items-center text-secondary hover:text-primary disabled:opacity-50"
			>
				{#if statusLoading}
					<Loader2 size={14} class="animate-spin" />
				{:else}
					<RefreshCw size={14} />
				{/if}
			</button>
		{/snippet}
		{#if status}
			<div class="flex flex-col gap-2">
				{#each [{ label: 'Job indexer', entry: status.job_indexer }, { label: 'Service log indexer', entry: status.log_indexer }] as { label, entry } (label)}
					<div class="flex flex-row items-center gap-2 text-xs">
						<span
							class="inline-block w-2 h-2 rounded-full {entry?.is_alive
								? 'bg-green-500'
								: 'bg-red-500'}"
						></span>
						<span class="text-primary font-medium">{label}:</span>
						<span class="font-semibold {entry?.is_alive ? 'text-green-600 dark:text-green-400' : 'text-red-600 dark:text-red-400'}">
							{entry?.is_alive ? 'Running' : 'Stopped'}
						</span>
						{#if entry?.last_locked_at}
							<span class="text-tertiary text-2xs">
								Last active: {formatTimeAgo(entry.last_locked_at)}
							</span>
						{/if}
					</div>
				{/each}
			</div>
		{:else if statusError && !statusLoading}
			<span class="text-2xs text-secondary">
				Could not fetch indexer status. Search may not be enabled in this build.
			</span>
		{/if}
	</Label>
	<Label label="Index storage">
		{#if status}
			<div class="flex flex-col gap-1 text-2xs text-tertiary">
				<span>
					Jobs index:
					{#if status.job_indexer?.storage?.disk_size_bytes != null}
						Disk: {displaySize(status.job_indexer.storage.disk_size_bytes) ?? 'N/A'}
					{/if}
					{#if status.job_indexer?.storage?.s3_size_bytes != null}
						{#if status.job_indexer?.storage?.disk_size_bytes != null}&middot;{/if}
						S3: {displaySize(status.job_indexer.storage.s3_size_bytes) ?? 'N/A'}
					{/if}
				</span>
				<span>
					Service logs index:
					{#if status.log_indexer?.storage?.disk_size_bytes != null}
						Disk: {displaySize(status.log_indexer.storage.disk_size_bytes) ?? 'N/A'}
					{/if}
					{#if status.log_indexer?.storage?.s3_size_bytes != null}
						{#if status.log_indexer?.storage?.disk_size_bytes != null}&middot;{/if}
						S3: {displaySize(status.log_indexer.storage.s3_size_bytes) ?? 'N/A'}
					{/if}
				</span>
			</div>
		{/if}
	</Label>
	<Label label="Clear index">
		<span class="text-xs text-secondary"
			>These buttons will clear the whole index, and the service will start reindexing from scratch.
			Full text search might be down during this time.</span
		>
		<div class="flex flex-col gap-3">
			<div class="flex flex-row items-center gap-2">
				<Button
					variant="default"
					unifiedSize="sm"
					onclick={() => {
						clearJobsIndexModalOpen = true
					}}
				>
					Clear jobs index
				</Button>
			</div>
			<div class="flex flex-row items-center gap-2">
				<Button
					variant="default"
					unifiedSize="sm"
					onclick={() => {
						clearServiceLogsIndexModalOpen = true
					}}
				>
					Clear service logs index
				</Button>
			</div>
		</div>
	</Label>
	<ConfirmationModal
		title="Clear jobs index"
		confirmationText="Clear"
		open={clearJobsIndexModalOpen}
		type="danger"
		on:canceled={() => {
			clearJobsIndexModalOpen = false
		}}
		on:confirmed={async () => {
			const r = await IndexSearchService.clearIndex({
				idxName: 'JobIndex'
			})
			sendUserToast(r)
			clearJobsIndexModalOpen = false
		}}
	>
		Are you sure you want to clear the jobs index? The service will start reindexing from scratch.
		Full text search might be down during this time.
	</ConfirmationModal>
	<ConfirmationModal
		title="Clear service logs index"
		confirmationText="Clear"
		open={clearServiceLogsIndexModalOpen}
		type="danger"
		on:canceled={() => {
			clearServiceLogsIndexModalOpen = false
		}}
		on:confirmed={async () => {
			const r = await IndexSearchService.clearIndex({
				idxName: 'ServiceLogIndex'
			})
			sendUserToast(r)
			clearServiceLogsIndexModalOpen = false
		}}
	>
		Are you sure you want to clear the service logs index? The service will start reindexing from
		scratch. Full text search might be down during this time.
	</ConfirmationModal>
</div>
