<script lang="ts">
	import Tooltip from '../Tooltip.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import type { Writable } from 'svelte/store'

	interface Props {
		values: Writable<Record<string, any>>
		disabled?: boolean
	}

	let { values, disabled = false }: Props = $props()
</script>

<div class="space-y-6">
	<div class="flex flex-col gap-1">
		<label for="commit_job_max_batch_size" class="block text-xs font-semibold text-emphasis">
			Commit max batch size <Tooltip>
				The max amount of documents (here jobs) per commit. To optimize indexing throughput, it is
				best to keep this as high as possible. However, especially when reindexing the whole
				instance, it can be useful to have a limit on how many jobs can be written without being
				committed. A commit will make the jobs available for search, constitute a "checkpoint" state
				in the indexing and will be logged.
			</Tooltip>
		</label>
		<TextInput
			inputProps={{
				type: 'number',
				placeholder: '100000',
				id: 'commit_job_max_batch_size',
				disabled
			}}
			bind:value={$values['indexer_settings'].commit_job_max_batch_size}
		/>
	</div>
	<div class="flex flex-col gap-1">
		<label for="refresh_index_period" class="block text-xs font-semibold text-emphasis">
			Refresh index period (s) <Tooltip>
				The index will query new jobs periodically and write them on the index. This setting sets
				that period.
			</Tooltip></label
		>
		<TextInput
			inputProps={{
				type: 'number',
				placeholder: '300',
				id: 'refresh_index_period',
				disabled
			}}
			bind:value={$values['indexer_settings'].refresh_index_period}
		/>
	</div>
	<div class="flex flex-col gap-1">
		<label for="max_indexed_job_log_size" class="block text-xs font-semibold text-emphasis">
			Max indexed job log size (KB) <Tooltip>
				Job logs are included when indexing, but to avoid the index size growing artificially, the
				logs will be truncated after a size has been reached.
			</Tooltip>
		</label>
		<TextInput
			inputProps={{
				type: 'number',
				placeholder: '1024',
				id: 'max_indexed_job_log_size',
				disabled,
				oninput: (e) => {
					if (e.target instanceof HTMLInputElement) {
						if (e.target.valueAsNumber) {
							$values['indexer_settings'].max_indexed_job_log_size =
								e.target.valueAsNumber * 1024
						}
					}
				}
			}}
			value={$values['indexer_settings'].max_indexed_job_log_size / 1024}
		/>
	</div>
</div>
