<script lang="ts">
	import { ExternalLink, Globe } from 'lucide-svelte'
	import { JobService, type Job } from '$lib/gen'
	import { base } from '$lib/base'
	import { workspaceStore } from '$lib/stores'
	import { msToReadableTimeShort } from '$lib/utils'
	import ChatCollapsibleCard from './copilot/chat/ChatCollapsibleCard.svelte'
	import ToolContentDisplay from './copilot/chat/ToolContentDisplay.svelte'
	import WebSearchSourcesDisplay from './copilot/chat/WebSearchSourcesDisplay.svelte'
	import GfmMarkdown from './GfmMarkdown.svelte'
	import { buildTranscript, type TranscriptEntry } from './agentTranscript'
	import type { AgentMessage } from './aiAgentResult'
	import { SvelteMap, SvelteSet } from 'svelte/reactivity'

	interface Props {
		messages: AgentMessage[]
		workspaceId?: string
	}

	let { messages, workspaceId }: Props = $props()

	let entries = $derived(buildTranscript(messages))

	let expanded = new SvelteSet<number>()
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
				await JobService.getJob({ id: jobId, workspace: workspaceId ?? $workspaceStore! })
			)
		} catch {
			// A tool job can be gone (retention) or unreadable. The row still has its
			// arguments and result from the envelope, so only the extras are lost.
			jobs.set(jobId, 'failed')
		}
	}

	function toggle(index: number, entry: TranscriptEntry) {
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

	function toolLabel(entry: Extract<TranscriptEntry, { kind: 'tool' }>): string {
		const job = jobOf(entry.jobId)
		const duration = job?.['duration_ms']
		return duration === undefined ? entry.name : `${entry.name} · ${msToReadableTimeShort(duration)}`
	}
</script>

<div class="flex flex-col w-full min-w-0">
	{#each entries as entry, index (index)}
		{#if entry.kind === 'user'}
			<!-- The session chat's user bubble, so a run reads the same here as it does
			     in a chat. Read-only: there is nothing to edit on a finished run. -->
			<div class="py-1 px-2 mt-4 mb-6 first:mt-0">
				<div
					class="text-xs px-3 py-2 w-fit max-w-[min(32rem,100%)] bg-surface-accent-selected text-accent rounded-lg whitespace-pre-wrap break-words"
				>
					{entry.content}
				</div>
			</div>
		{:else if entry.kind === 'assistant'}
			<div class="py-1 px-2 min-w-0">
				<GfmMarkdown md={entry.content} noPadding />
				{#if entry.sources}
					<div class="mt-2">
						<WebSearchSourcesDisplay sources={entry.sources} />
					</div>
				{/if}
			</div>
		{:else if entry.kind === 'system'}
			<div class="px-2 mb-1">
				<ChatCollapsibleCard
					label="System prompt"
					expanded={expanded.has(index)}
					onToggle={() => toggle(index, entry)}
				>
					<div class="whitespace-pre-wrap text-2xs text-primary">{entry.content}</div>
				</ChatCollapsibleCard>
			</div>
		{:else if entry.kind === 'search'}
			<div class="px-2 mb-1">
				<ChatCollapsibleCard
					label="Web search"
					expanded={expanded.has(index)}
					onToggle={() => toggle(index, entry)}
				>
					{#if entry.sources}
						<WebSearchSourcesDisplay sources={entry.sources} />
					{:else}
						<ToolContentDisplay title="Result" content={entry.content} />
					{/if}
				</ChatCollapsibleCard>
			</div>
		{:else}
			{@const job = jobOf(entry.jobId)}
			<div class="px-2 mb-1">
				<ChatCollapsibleCard
					label={toolLabel(entry)}
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
					<ToolContentDisplay
						title="Result"
						content={entry.result}
						error={job?.type === 'CompletedJob' && !job.success ? entry.result : undefined}
					/>
					{#if entry.resourcePath}
						<div class="text-2xs text-hint flex items-center gap-1">
							<Globe size={11} />
							{entry.resourcePath}
						</div>
					{:else if entry.jobId}
						<a
							class="text-2xs text-accent inline-flex items-center gap-1 w-fit hover:underline"
							href="{base}/run/{entry.jobId}?workspace={workspaceId ?? $workspaceStore}"
							target="_blank"
							rel="noreferrer"
						>
							<ExternalLink size={11} />
							Open job
						</a>
					{/if}
				</ChatCollapsibleCard>
			</div>
		{/if}
	{/each}
</div>
