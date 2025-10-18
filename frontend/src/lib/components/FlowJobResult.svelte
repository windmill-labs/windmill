<script lang="ts">
	import { Loader2 } from 'lucide-svelte'
	import DisplayResult from './DisplayResult.svelte'
	import LogViewer from './LogViewer.svelte'
	import type { CompletedJob, Job } from '$lib/gen'
	import AiAgentLogViewer from './AIAgentLogViewer.svelte'
	import type { AgentTool } from './flows/agentToolUtils'

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
		aiAgentStatus?: {
			tools: AgentTool[]
			agentJob: Partial<CompletedJob> & Pick<CompletedJob, 'id'> & { type: 'CompletedJob' }
			storedToolCallJobs?: Record<number, Job>
			onToolJobLoaded?: (job: Job, idx: number) => void
		}
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
		tagLabel = undefined,
		aiAgentStatus = undefined
	}: Props = $props()
</script>

<div
	class:border={!noBorder}
	class="{!col
		? 'grid grid-cols-2'
		: 'flex flex-col max-h-screen gap-4'} shadow border border-tertiary-inverse grow overflow-hidden"
>
	<div class="bg-surface {col ? 'max-h-1/2 grow' : 'max-h-80'} p-1 overflow-auto relative">
		<span class="text-tertiary">Result</span>
		{#if result !== undefined || result_stream !== undefined}
			<DisplayResult {workspaceId} {jobId} {filename} {result} {result_stream} growVertical />
		{:else if loading}
			<Loader2 class="animate-spin" />
		{:else}
			<div class="text-gray-400">No result (result is undefined)</div>
		{/if}
	</div>
	<div class="overflow-auto {col ? 'grow' : 'max-h-80'} relative">
		{#if aiAgentStatus}
			<AiAgentLogViewer {...aiAgentStatus} {workspaceId} />
		{:else}
			<LogViewer
				{tagLabel}
				download={downloadLogs}
				content={logs ?? ''}
				{jobId}
				isLoading={waitingForExecutor}
				{tag}
			/>
		{/if}
	</div>
</div>
