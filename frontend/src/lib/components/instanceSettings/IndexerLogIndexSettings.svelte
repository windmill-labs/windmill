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
		<TextInput
			inputProps={{
				type: 'number',
				placeholder: '10000',
				id: 'commit_log_max_batch_size',
				disabled
			}}
			bind:value={$values['indexer_settings'].commit_log_max_batch_size}
		/>
	</div>

	<div class="flex flex-col gap-1">
		<label for="refresh_log_index_period" class="block text-xs font-semibold text-emphasis">
			Refresh index period (s) <Tooltip>
				The index will query new service logs peridically and write them on the index. This setting
				sets that period.
			</Tooltip>
		</label>
		<TextInput
			inputProps={{
				type: 'number',
				placeholder: '300',
				id: 'refresh_log_index_period',
				disabled
			}}
			bind:value={$values['indexer_settings'].refresh_log_index_period}
		/>
	</div>
</div>
