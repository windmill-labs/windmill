<script lang="ts">
	import Tooltip from '../Tooltip.svelte'
	import IntegerInput from '../IntegerInput.svelte'
	import InputError from '../InputError.svelte'
	import type { Writable } from 'svelte/store'

	interface Props {
		values: Writable<Record<string, any>>
		disabled?: boolean
		errors?: Record<string, string>
	}

	let { values, disabled = false, errors = {} }: Props = $props()
</script>

<div class="space-y-6">
	<div class="flex flex-col gap-1">
		<label for="commit_log_max_batch_size" class="block text-xs font-semibold text-emphasis"
			>Commit max batch size <Tooltip>
				The max amount of documents per commit. In this case 1 document is one log file representing
				all logs during 1 minute for a specific host. To optimize indexing throughput, it is best to
				keep this as high as possible. However, especially when reindexing the whole instance, it
				can be useful to have a limit on how many logs can be written without being committed. A
				commit will make the logs available for search, appear as a log line, and be a "checkpoint"
				of the indexing progress.
			</Tooltip>
		</label>
		<IntegerInput
			placeholder="10000"
			id="commit_log_max_batch_size"
			{disabled}
			error={errors.commit_log_max_batch_size ?? ''}
			value={$values['indexer_settings'].commit_log_max_batch_size}
			oninput={(v) => {
				if (v == null) {
					const { commit_log_max_batch_size: _, ...rest } = $values['indexer_settings']
					$values['indexer_settings'] = rest
				} else {
					$values['indexer_settings'] = {
						...$values['indexer_settings'],
						commit_log_max_batch_size: v
					}
				}
			}}
		/>
		<InputError error={errors.commit_log_max_batch_size ?? ''} />
	</div>

	<div class="flex flex-col gap-1">
		<label for="refresh_log_index_period" class="block text-xs font-semibold text-emphasis">
			Refresh index period (s) <Tooltip>
				The index will query new service logs peridically and write them on the index. This setting
				sets that period.
			</Tooltip>
		</label>
		<IntegerInput
			placeholder="300"
			id="refresh_log_index_period"
			{disabled}
			error={errors.refresh_log_index_period ?? ''}
			value={$values['indexer_settings'].refresh_log_index_period}
			oninput={(v) => {
				if (v == null) {
					const { refresh_log_index_period: _, ...rest } = $values['indexer_settings']
					$values['indexer_settings'] = rest
				} else {
					$values['indexer_settings'] = {
						...$values['indexer_settings'],
						refresh_log_index_period: v
					}
				}
			}}
		/>
		<InputError error={errors.refresh_log_index_period ?? ''} />
	</div>
</div>
