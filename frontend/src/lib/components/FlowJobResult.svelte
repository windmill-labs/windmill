<script lang="ts">
	import { Loader2 } from 'lucide-svelte'
	import DisplayResult from './DisplayResult.svelte'
	import LogViewer from './LogViewer.svelte'

	interface Props {
		waitingForExecutor?: boolean
		result: any
		result_stream?: string
		logs: string | undefined
		col?: boolean
		noBorder?: boolean
		loading: boolean
		filename?: string | undefined
		jobId?: string | undefined
		tag?: string | undefined
		workspaceId?: string | undefined
		refreshLog?: boolean
		downloadLogs?: boolean
		tagLabel?: string | undefined
	}

	let {
		waitingForExecutor = false,
		result,
		result_stream,
		logs,
		col = false,
		noBorder = false,
		loading,
		filename = undefined,
		jobId = undefined,
		tag = undefined,
		workspaceId = undefined,
		downloadLogs = true,
		tagLabel = undefined
	}: Props = $props()
</script>

<div
	class:border={!noBorder}
	class="grid {!col
		? 'grid-cols-2'
		: 'grid-rows-2 max-h-screen'} shadow border border-tertiary-inverse grow overflow-hidden"
>
	<div class="bg-surface {col ? '' : 'max-h-80'} p-1 overflow-auto relative">
		<span class="text-tertiary">Result</span>
		{#if result !== undefined || result_stream !== undefined}
			<DisplayResult {workspaceId} {jobId} {filename} {result} {result_stream} />
		{:else if loading}
			<Loader2 class="animate-spin" />
		{:else}
			<div class="text-gray-400">No result (result is undefined)</div>
		{/if}
	</div>
	<div class="overflow-auto {col ? '' : 'max-h-80'} relative">
		<LogViewer
			{tagLabel}
			download={downloadLogs}
			content={logs ?? ''}
			{jobId}
			isLoading={waitingForExecutor}
			{tag}
		/>
	</div>
</div>
