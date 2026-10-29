<script lang="ts">
	import { Loader2 } from 'lucide-svelte'
	import DisplayResult from './DisplayResult.svelte'
	import LogViewer from './LogViewer.svelte'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		waitingForExecutor?: boolean
		result: any
		result_stream?: string
		logs: string | undefined
		col?: boolean
		loading: boolean
		filename?: string | undefined
		jobId?: string | undefined
		/** Identifies the run, which `jobId` cannot on the replay page. */
		runKey?: string | undefined
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
		loading,
		filename = undefined,
		jobId = undefined,
		runKey = undefined,
		tag = undefined,
		workspaceId = undefined,
		downloadLogs = true,
		tagLabel = undefined
	}: Props = $props()
</script>

<div
	class={twMerge(
		'grow text-xs',
		!col ? 'grid grid-cols-2 gap-2 overflow-hidden' : 'flex flex-col gap-6 min-h-0'
	)}
>
	<div class="relative flex flex-col gap-1 {col ? 'min-h-[120px] max-h-[50%]' : ''}">
		<span class="text-emphasis text-xs font-semibold">Result</span>
		<div
			class="{col
				? 'max-h-1/2 grow'
				: 'max-h-80'} overflow-auto rounded-md grow min-h-0 border bg-surface-tertiary p-2"
		>
			{#if result !== undefined || result_stream !== undefined}
				<DisplayResult
					{workspaceId}
					{jobId}
					{runKey}
					{filename}
					{result}
					{result_stream}
					growVertical
				/>
			{:else if loading}
				<Loader2 class="animate-spin" />
			{:else}
				<div class="text-secondary">No result (result is undefined)</div>
			{/if}
		</div>
	</div>
	<div class="relative flex flex-col gap-1 {col ? 'flex-1 min-h-[200px]' : ''}">
		<span class="text-emphasis text-xs font-semibold">Logs</span>
		<div class="rounded-md grow min-h-0 border bg-surface-tertiary overflow-hidden">
			<!-- In col mode the caller gives this column a definite height and the log box fills it:
			     a max-h cap of its own would overflow the column and get its tail clipped -->
			<LogViewer
				noMaxH={col}
				{tagLabel}
				download={downloadLogs}
				content={logs ?? ''}
				{jobId}
				isLoading={waitingForExecutor}
				{tag}
			/>
		</div>
	</div>
</div>
