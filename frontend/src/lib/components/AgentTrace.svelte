<script lang="ts">
	import { ExternalLink, Globe } from 'lucide-svelte'
	import { JobService, type Job } from '$lib/gen'
	import { base } from '$lib/base'
	import { msToReadableTimeShort } from '$lib/utils'
	import ChatCollapsibleCard from './copilot/chat/ChatCollapsibleCard.svelte'
	import ToolContentDisplay from './copilot/chat/ToolContentDisplay.svelte'
	import WebSearchSourcesDisplay from './copilot/chat/WebSearchSourcesDisplay.svelte'
	import { webSearchResultOf, type WebSearchResult } from './copilot/chat/shared'
	import GfmMarkdown from './GfmMarkdown.svelte'
	import type { AgentTraceEntry } from './agentTrace'
	import { SvelteMap, SvelteSet } from 'svelte/reactivity'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'

	const operatingWorkspace = useOperatingWorkspace()

	interface Props {
		entries: AgentTraceEntry[]
		workspaceId?: string
	}

	let { entries, workspaceId }: Props = $props()

	let expanded = new SvelteSet<number>()
	// A search opens on its sources, so for it the reader's toggle records a close.
	let collapsedSearches = new SvelteSet<number>()
	// Row state is keyed by position, which means nothing once the viewer is handed
	// another run: row 0 would stay open showing the previous run's job.
	$effect(() => {
		entries
		expanded.clear()
		collapsedSearches.clear()
		jobs.clear()
	})
	// A tool's own job holds what the envelope does not: its logs, how long it
	// took, and whether it succeeded. Fetched when a row is opened rather than
	// upfront, so a run with twenty calls does not issue twenty requests to draw
	// a list of names.
	let jobs = new SvelteMap<string, Job | 'loading' | 'failed'>()

	async function loadJob(jobId: string) {
		if (jobs.has(jobId)) {
			return
		}
		jobs.set(jobId, 'loading')
		try {
			jobs.set(
				jobId,
				await JobService.getJob({ id: jobId, workspace: workspaceId ?? $operatingWorkspace! })
			)
		} catch {
			// A tool job can be gone (retention) or unreadable. The row still has its
			// arguments and result from the envelope, so only the extras are lost.
			jobs.set(jobId, 'failed')
		}
	}

	function toggle(index: number, entry: AgentTraceEntry) {
		if (expanded.delete(index)) {
			return
		}
		expanded.add(index)
		if (entry.kind === 'tool' && entry.jobId) {
			loadJob(entry.jobId)
		}
	}

	function jobOf(jobId: string | undefined): Job | undefined {
		if (!jobId) return undefined
		const job = jobs.get(jobId)
		return typeof job === 'object' ? job : undefined
	}

	function toolLabel(
		entry: Extract<AgentTraceEntry, { kind: 'tool' }>,
		search: WebSearchResult | undefined
	): string {
		const duration = jobOf(entry.jobId)?.['duration_ms']
		return [
			entry.name,
			search?.query !== undefined ? `"${search.query}"` : undefined,
			duration !== undefined ? msToReadableTimeShort(duration) : undefined
		]
			.filter((part) => part !== undefined)
			.join(' · ')
	}
</script>

<div class="flex flex-col w-full min-w-0 gap-1">
	{#each entries as entry, index (index)}
		{#if entry.kind === 'assistant'}
			<div class="min-w-0">
				<GfmMarkdown md={entry.content} noPadding />
			</div>
		{:else if entry.kind === 'search'}
			<ChatCollapsibleCard
				label="Searched the web"
				expanded={Boolean(entry.sources) && !collapsedSearches.has(index)}
				toggleable={Boolean(entry.sources)}
				onToggle={() => {
					if (!collapsedSearches.delete(index)) collapsedSearches.add(index)
				}}
			>
				{#if entry.sources}
					<WebSearchSourcesDisplay sources={entry.sources} />
				{/if}
			</ChatCollapsibleCard>
		{:else}
			{@const job = jobOf(entry.jobId)}
			{@const failed = job?.type === 'CompletedJob' && !job.success}
			{@const search = failed ? undefined : webSearchResultOf(entry.result)}
			<ChatCollapsibleCard
				label={toolLabel(entry, search)}
				expanded={expanded.has(index)}
				onToggle={() => toggle(index, entry)}
				contentClass="space-y-3"
			>
				{#if entry.args}
					<ToolContentDisplay title="Parameters" content={entry.args} toolName={entry.name} />
				{/if}
				{#if job?.logs}
					<ToolContentDisplay title="Logs" content={job.logs} />
				{/if}
				{#if search}
					<WebSearchSourcesDisplay sources={search.sources} />
				{:else}
					<ToolContentDisplay
						title="Result"
						content={entry.result}
						error={failed ? entry.result : undefined}
					/>
				{/if}
				{#if entry.resourcePath}
					<div class="text-2xs text-hint flex items-center gap-1">
						<Globe size={11} />
						{entry.resourcePath}
					</div>
				{:else if entry.jobId}
					<a
						class="text-2xs text-accent inline-flex items-center gap-1 w-fit hover:underline"
						href="{base}/run/{entry.jobId}?workspace={workspaceId ?? $operatingWorkspace}"
						target="_blank"
						rel="noreferrer"
					>
						<ExternalLink size={11} />
						Open job
					</a>
				{/if}
			</ChatCollapsibleCard>
		{/if}
	{/each}
</div>
