<script lang="ts">
	import { base } from '$lib/base'
	import { workspaceStore } from '$lib/stores'
	import type { Job } from '$lib/gen'
	import Select from '$lib/components/select/Select.svelte'
	import JobLoader from '$lib/components/JobLoader.svelte'
	import LogViewer from '$lib/components/LogViewer.svelte'
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import JobArgs from '$lib/components/JobArgs.svelte'
	import {
		CheckCircle2,
		ChevronDown,
		ChevronRight,
		Clock,
		Code2,
		GitBranch,
		History,
		Loader2,
		XCircle
	} from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { untrack } from 'svelte'
	import { isActiveEvent, type PipelineEvent } from './activeRunnables.svelte'

	interface Props {
		// Merged history + live events, newest-first (page owns the merge —
		// the live poll wins on id collisions since it carries fresher status).
		events: PipelineEvent[]
		loading?: boolean
		// History preload hit its page cap before the days cutoff.
		truncated?: boolean
		error?: string | undefined
		days: number
		onDaysChange: (days: number) => void
	}

	let { events, loading = false, truncated = false, error, days, onDaysChange }: Props = $props()

	const DAY_OPTIONS = [
		{ label: 'Last 7 days', value: 7 },
		{ label: 'Last 30 days', value: 30 },
		{ label: 'Last 90 days', value: 90 }
	]

	// Excludes future-scheduled queued jobs (a schedule's next planned run
	// is not activity) — see isActiveEvent.
	let runningCount = $derived(events.filter((e) => isActiveEvent(e)).length)

	// Accordion: one expanded run at a time, streamed via JobLoader (live
	// logs while running, terminal result when done) — same pattern as
	// AssetRunsPanel's selected job.
	let expandedId = $state<string | undefined>(undefined)
	let expandedJob = $state<(Job & { logs?: string; result?: any }) | undefined>(undefined)
	let jobLoader: JobLoader | undefined = $state()
	let jobLoaderLoading = $state(false)
	// Only (re)start the JobLoader stream when the expanded id actually
	// changes — watchJob resets internal state and refetches on every call
	// (cf. the request-flood note in AssetRunsPanel).
	let lastWatchedId = $state<string | undefined>(undefined)
	$effect(() => {
		const id = expandedId
		if (id === lastWatchedId) return
		lastWatchedId = id
		if (!id) {
			untrack(() => {
				expandedJob = undefined
				void jobLoader?.clearCurrentJob?.()
			})
			return
		}
		untrack(() => {
			void jobLoader?.watchJob(id)
		})
	})

	function toggle(id: string) {
		expandedId = expandedId === id ? undefined : id
	}

	function isRunningJob(j: Job): boolean {
		return j.type === 'QueuedJob' && (j as any).running === true
	}
	function jobStatus(j: Job): 'running' | 'queued' | 'success' | 'failure' | 'canceled' {
		if (isRunningJob(j)) return 'running'
		if (j.type === 'QueuedJob') return 'queued'
		if ((j as any).canceled) return 'canceled'
		if ((j as any).success === false) return 'failure'
		return 'success'
	}
	function durationMs(j: Job): number | undefined {
		const started = (j as any).started_at as string | undefined
		const created = (j as any).created_at as string | undefined
		if (isRunningJob(j) && started) {
			return Date.now() - new Date(started).getTime()
		}
		const dms = (j as any).duration_ms as number | undefined
		if (dms != undefined) return dms
		const ended = (j as any).ended_at as string | undefined
		if (started && ended) return new Date(ended).getTime() - new Date(started).getTime()
		if (created && ended) return new Date(ended).getTime() - new Date(created).getTime()
		return undefined
	}

	function ago(iso: string): string {
		const s = Math.max(0, Math.floor((Date.now() - new Date(iso).getTime()) / 1000))
		if (s < 60) return `${s}s ago`
		if (s < 3600) return `${Math.floor(s / 60)}m ago`
		if (s < 86400) return `${Math.floor(s / 3600)}h ago`
		return `${Math.floor(s / 86400)}d ago`
	}

	// A schedule's next planned run sits in the queue with a future
	// scheduled_for — show when it's due instead of a misleading "ago"
	// (its `at` is the enqueue time).
	function isFutureScheduled(e: PipelineEvent): boolean {
		return e.status === 'queued' && !isActiveEvent(e)
	}
	function inTxt(iso: string): string {
		const s = Math.max(0, Math.floor((new Date(iso).getTime() - Date.now()) / 1000))
		if (s < 60) return `in ${s}s`
		if (s < 3600) return `in ${Math.floor(s / 60)}m`
		if (s < 86400) return `in ${Math.floor(s / 3600)}h`
		return `in ${Math.floor(s / 86400)}d`
	}
</script>

<JobLoader bind:this={jobLoader} bind:job={expandedJob} bind:isLoading={jobLoaderLoading} />

<div class="flex flex-col h-full bg-surface">
	<div class="flex items-center justify-between gap-2 px-3 py-2 border-b shrink-0 min-h-10">
		<span class="flex items-center gap-2 text-xs font-semibold text-emphasis">
			<History size={14} />
			Activity
			{#if runningCount > 0}
				<span
					class="flex items-center gap-1 px-1.5 py-0.5 rounded-full bg-blue-100 dark:bg-blue-900/40 text-blue-700 dark:text-blue-300 text-2xs font-normal"
				>
					<Loader2 size={9} class="animate-spin" />
					{runningCount} running
				</span>
			{:else if loading}
				<Loader2 size={12} class="animate-spin text-tertiary" />
			{/if}
		</span>
		<div class="w-36 shrink-0">
			<Select
				items={DAY_OPTIONS}
				size="sm"
				clearable={false}
				bind:value={() => days, (v) => onDaysChange(v ?? 30)}
			/>
		</div>
	</div>

	<div class="flex-1 min-h-0 overflow-y-auto px-1 py-1">
		{#if error}
			<div class="px-3 py-4 text-xs text-red-600 dark:text-red-400">
				Failed to load activity: {error}
			</div>
		{:else if events.length === 0}
			{#if loading}
				<div class="flex items-center justify-center gap-2 px-3 py-6 text-xs text-tertiary">
					<Loader2 size={13} class="animate-spin" /> Loading activity…
				</div>
			{:else}
				<div class="px-3 py-6 text-center text-xs text-tertiary">
					No runs in the last {days} days — executions of this pipeline will appear here live.
				</div>
			{/if}
		{:else}
			{#each events as e (e.id)}
				{@const expanded = expandedId === e.id}
				<button
					type="button"
					onclick={() => toggle(e.id)}
					class={twMerge(
						'w-full flex items-center gap-2 px-2 py-1 rounded-sm hover:bg-surface-hover text-xs text-left',
						expanded && 'bg-surface-selected'
					)}
					title={expanded ? 'Collapse run details' : 'Expand run details'}
				>
					<span class="shrink-0 text-tertiary">
						{#if expanded}
							<ChevronDown size={12} />
						{:else}
							<ChevronRight size={12} />
						{/if}
					</span>
					<span class="shrink-0">
						{#if e.status === 'running'}
							<Loader2 size={12} class="animate-spin text-blue-600 dark:text-blue-400" />
						{:else if e.status === 'queued'}
							<Clock size={12} class="text-gray-500" />
						{:else if e.status === 'success'}
							<CheckCircle2 size={12} class="text-emerald-600 dark:text-emerald-400" />
						{:else}
							<XCircle size={12} class="text-red-600 dark:text-red-400" />
						{/if}
					</span>
					{#if e.kind === 'flow'}
						<GitBranch size={12} class="shrink-0 text-emerald-700 dark:text-emerald-400" />
					{:else}
						<Code2 size={12} class="shrink-0 text-emerald-700 dark:text-emerald-400" />
					{/if}
					<span class="flex-1 min-w-0 truncate font-mono text-2xs" title={e.path}>
						{e.path}
					</span>
					<span
						class={twMerge(
							'shrink-0 px-1 py-0.5 rounded-sm text-3xs leading-none',
							e.source === 'schedule'
								? 'bg-amber-100 dark:bg-amber-900/40 text-amber-700 dark:text-amber-300'
								: 'bg-gray-100 dark:bg-gray-800 text-gray-600 dark:text-gray-400'
						)}
					>
						{e.source}
					</span>
					<span class="shrink-0 text-3xs text-tertiary tabular-nums w-14 text-right">
						{isFutureScheduled(e) && e.scheduledFor ? inTxt(e.scheduledFor) : ago(e.at)}
					</span>
				</button>
				{#if expanded}
					<div class="mx-2 mb-1 border rounded-md bg-surface-secondary/30">
						{#if !expandedJob && jobLoaderLoading}
							<div class="flex items-center gap-1 p-3 text-xs text-tertiary">
								<Loader2 size={12} class="animate-spin" /> Loading run…
							</div>
						{:else if expandedJob}
							{@const status = jobStatus(expandedJob)}
							<div class="flex flex-col gap-2 p-2">
								<div class="flex items-center gap-2 text-2xs text-tertiary">
									<span class="font-mono truncate flex-1 min-w-0">{expandedJob.id}</span>
									<a
										class="text-blue-600 hover:underline shrink-0"
										href={`${base}/run/${expandedJob.id}?workspace=${$workspaceStore}`}
										target="_blank">Open ↗</a
									>
								</div>
								<JobArgs
									id={expandedJob.id}
									workspace={expandedJob.workspace_id ?? $workspaceStore ?? ''}
									args={expandedJob.args}
								/>
								<div class="flex flex-col gap-1">
									<div class="text-3xs uppercase tracking-wide text-tertiary">Logs</div>
									<LogViewer
										jobId={expandedJob.id}
										duration={durationMs(expandedJob)}
										mem={(expandedJob as any).mem_peak}
										tag={(expandedJob as any).tag}
										isLoading={jobLoaderLoading && !expandedJob.logs}
										content={expandedJob.logs}
										download={false}
									/>
								</div>
								{#if status === 'success' || status === 'failure'}
									<div class="flex flex-col gap-1">
										<div class="text-3xs uppercase tracking-wide text-tertiary">
											{status === 'failure' ? 'Error' : 'Result'}
										</div>
										<DisplayResult
											workspaceId={expandedJob.workspace_id}
											jobId={expandedJob.id}
											result={expandedJob.result}
										/>
									</div>
								{/if}
							</div>
						{:else}
							<div class="p-3 text-xs text-tertiary">Could not load this run.</div>
						{/if}
					</div>
				{/if}
			{/each}
			{#if truncated}
				<div class="px-3 py-2 text-center text-2xs text-tertiary">
					Showing the latest {events.length} runs — older runs in this window are not listed.
				</div>
			{/if}
		{/if}
	</div>
</div>
