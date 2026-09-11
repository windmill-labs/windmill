<script lang="ts">
	import { Loader2 } from 'lucide-svelte'
	import DisplayResult from './DisplayResult.svelte'
	import LogViewer from './LogViewer.svelte'
	import { twMerge } from 'tailwind-merge'
	import AgentTranscript from './AgentTranscript.svelte'
	import { parseAgentResult } from './aiAgentResult'

	interface Props {
		waitingForExecutor?: boolean
		result: any
		result_stream?: string
		logs: string | undefined
		col?: boolean
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
		loading,
		filename = undefined,
		jobId = undefined,
		tag = undefined,
		workspaceId = undefined,
		downloadLogs = true,
		tagLabel = undefined
	}: Props = $props()

	// An agent step's own logs are worker chatter; what happened is its conversation.
	// Derived from the result rather than passed in, so every caller gets it.
	let agentResult = $derived(parseAgentResult(result))
</script>

<div
	class={twMerge(
		'grow overflow-hidden text-xs',
		!col ? 'grid grid-cols-2 gap-2' : 'flex flex-col max-h-screen gap-6 overflow-hidden'
	)}
>
	<div class="relative flex flex-col gap-1">
		<span class="text-emphasis text-xs font-semibold">Result</span>
		<div
			class="{col
				? 'max-h-1/2 grow'
				: 'max-h-80'} overflow-auto rounded-md grow min-h-0 border bg-surface-tertiary p-2"
		>
			{#if result !== undefined || result_stream !== undefined}
				<DisplayResult {workspaceId} {jobId} {filename} {result} {result_stream} growVertical />
			{:else if loading}
				<Loader2 class="animate-spin" />
			{:else}
				<div class="text-secondary">No result (result is undefined)</div>
			{/if}
		</div>
	</div>
	<div class="relative flex flex-col gap-1">
		<span class="text-emphasis text-xs font-semibold">{agentResult ? 'Transcript' : 'Logs'}</span>
		{#if agentResult}
			<div class="rounded-md grow min-h-0 border bg-surface-tertiary overflow-auto p-2">
				<AgentTranscript messages={agentResult.messages} {workspaceId} />
			</div>
		{:else}
			<div class="rounded-md grow min-h-0 border bg-surface-tertiary overflow-hidden">
				<LogViewer
					{tagLabel}
					download={downloadLogs}
					content={logs ?? ''}
					{jobId}
					isLoading={waitingForExecutor}
					{tag}
				/>
			</div>
		{/if}
	</div>
</div>
