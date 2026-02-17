<script lang="ts">
	import { Button } from '$lib/components/common'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import { IndexSearchService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import Tooltip from '../Tooltip.svelte'
	import IntegerInput from '../IntegerInput.svelte'
	import InputError from '../InputError.svelte'
	import Label from '../Label.svelte'
	import type { Writable } from 'svelte/store'

	interface Props {
		values: Writable<Record<string, any>>
		disabled?: boolean
		errors?: Record<string, string>
	}

	let { values, disabled = false, errors = {} }: Props = $props()

	let clearJobsIndexModalOpen = $state(false)
	let clearServiceLogsIndexModalOpen = $state(false)
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
	<Label label="Clear index">
		<span class="text-xs text-secondary"
			>This buttons will clear the whole index, and the service will start reindexing from scratch.
			Full text search might be down during this time.</span
		>
		<div class="flex flex-row gap-2">
			<Button
				variant="default"
				unifiedSize="sm"
				on:click={() => {
					clearJobsIndexModalOpen = true
				}}
			>
				Clear jobs index
			</Button>
			<Button
				variant="default"
				unifiedSize="sm"
				on:click={() => {
					clearServiceLogsIndexModalOpen = true
				}}
			>
				Clear service logs index
			</Button>
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
