<script lang="ts">
	import { base } from '$lib/base'
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import AiAgentLogViewer from '$lib/components/AIAgentLogViewer.svelte'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import { Tab } from '$lib/components/common'
	import { JobService, type Job } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { displayDate } from '$lib/utils'
	import { ExternalLink } from 'lucide-svelte'
	import type { AgentTool } from '$lib/components/flows/agentToolUtils'

	let {
		job,
		tools = [],
		historyPath
	}: {
		job: (Job & { result?: any }) | undefined
		tools?: AgentTool[]
		/** Exact job path of this case's runs. Absent for an unsaved case, which has no history. */
		historyPath?: string
	} = $props()

	let tab = $state<'result' | 'trajectory' | 'history'>('result')

	// The run history is a plain jobs-list query on the path the run was stamped with — the runs
	// are the stored result, so there is no separate record to keep in step with them.
	let history = $state<Job[]>([])
	$effect(() => {
		const path = historyPath
		const workspace = $workspaceStore
		if (!path || !workspace || tab !== 'history') return
		JobService.listJobs({ workspace, scriptPathExact: path, perPage: 20 })
			.then((jobs) => (history = jobs))
			.catch(() => (history = []))
	})

	// The child AI agent job holds the trajectory; the wrapper flow job only holds the result.
	let agentJob = $derived(
		job?.type === 'CompletedJob' ? { ...job, type: 'CompletedJob' as const } : undefined
	)
</script>

<div class="flex flex-col h-full min-h-0">
	<Tabs bind:selected={tab}>
		<Tab value="result" label="Result" />
		<Tab value="trajectory" label="Trajectory" />
		{#if historyPath}
			<Tab value="history" label="History" />
		{/if}
	</Tabs>

	<div class="flex-1 min-h-0 overflow-auto pt-2">
		{#if tab === 'result'}
			{#if job?.type === 'CompletedJob'}
				<DisplayResult
					workspaceId={job.workspace_id}
					jobId={job.id}
					result={job.result}
					disableExpand
				/>
			{:else if job}
				<div class="text-xs text-tertiary">Running…</div>
			{:else}
				<div class="text-xs text-tertiary">Run the case to see its output here.</div>
			{/if}
		{:else if tab === 'trajectory'}
			{#if agentJob}
				<AiAgentLogViewer {tools} {agentJob} workspaceId={agentJob.workspace_id} noPadding />
			{:else}
				<div class="text-xs text-tertiary">No trajectory yet.</div>
			{/if}
		{:else if history.length === 0}
			<div class="text-xs text-tertiary">No previous run of this case.</div>
		{:else}
			<div class="flex flex-col divide-y">
				{#each history as run (run.id)}
					<a
						class="flex items-center justify-between gap-2 py-1.5 text-xs hover:bg-surface-hover"
						href={`${base}/run/${run.id}?workspace=${run.workspace_id}`}
						target="_blank"
					>
						<span class="truncate">
							{displayDate(run.created_at)} · {run.created_by}
						</span>
						<span class="flex items-center gap-1 shrink-0">
							{#if run.type === 'CompletedJob'}
								<span class={run.success ? 'text-green-600' : 'text-red-600'}>
									{run.success ? 'success' : 'failure'}
								</span>
							{:else}
								<span class="text-tertiary">running</span>
							{/if}
							<ExternalLink size={12} />
						</span>
					</a>
				{/each}
			</div>
		{/if}
	</div>
</div>
